use crate::{
  context::AppCtx,
  events::{
    dispatcher::Dispatcher,
    focus_mgr::{FocusManager, FocusType},
  },
  prelude::*,
  ticker::{FrameMsg, FrameTicker},
  widget::WidgetId,
  widget_tree::WidgetTree,
};
use futures::{
  task::{LocalSpawnExt, SpawnError},
  Future,
};
use ribir_geom::Point;
use rxrust::{scheduler::FuturesLocalScheduler, subject::Subject};
use std::{
  borrow::BorrowMut,
  cell::{Cell, RefCell},
  collections::VecDeque,
  convert::Infallible,
  ops::{Deref, DerefMut},
  rc::Rc,
  time::Instant,
};
use winit::event::WindowEvent;
pub use winit::window::CursorIcon;

/// Window is the root to represent.
///
/// We use `RefCell` to wrap every field of `Window` to make sure we can split
/// borrow the fields in runtime. So we can pass `Window` to user when the
/// framework borrower  one of the fields. e.g. `dispatcher` is borrowed when
/// dispatch the event, but user may access the `Window` to change the title in
/// event callback.
pub struct Window {
  pub(crate) painter: RefCell<Painter>,
  pub(crate) dispatcher: RefCell<Dispatcher>,
  pub(crate) widget_tree: RefCell<WidgetTree>,
  pub(crate) frame_ticker: FrameTicker,
  pub(crate) focus_mgr: RefCell<FocusManager>,
  pub(crate) running_animates: Rc<Cell<u32>>,
  /// This vector store the task to emit events. When perform layout, dispatch
  /// event and so on, some part of window may be already mutable borrowed and
  /// the user event callback may also query borrow that part, so we can't emit
  /// event immediately. So we store the event emitter in this vector,
  /// and emit them after all borrow finished.
  pub(crate) delay_emitter: RefCell<VecDeque<DelayEvent>>,
  /// A task pool use to process `Future` or `rxRust` task, and will block until
  /// all task finished before current frame end.
  frame_pool: RefCell<FuturesLocalSchedulerPool>,
  shell_wnd: RefCell<Box<dyn ShellWindow>>,
  /// A vector store the widget id pair of (parent, child). The child need to
  /// drop after its `DelayDrop::delay_drop_until` be false or its parent
  /// is dropped.
  ///
  /// This widgets it's detached from its parent, but still need to paint.
  delay_drop_widgets: RefCell<Vec<(Option<WidgetId>, WidgetId)>>,
  /// A hash set store the root of the subtree need to regenerate.
  regenerating_subtree: RefCell<ahash::HashMap<WidgetId, Option<WidgetId>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct WindowId(u64);

pub trait ShellWindow {
  fn id(&self) -> WindowId;
  fn inner_size(&self) -> Size;
  fn outer_size(&self) -> Size;
  fn set_ime_cursor_area(&mut self, rect: &Rect);

  fn request_resize(&mut self, size: Size);
  fn on_resize(&mut self, size: Size);
  fn set_min_size(&mut self, size: Size);
  fn cursor(&self) -> CursorIcon;
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn set_title(&mut self, str: &str);
  fn set_icon(&mut self, icon: &PixelImage);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  fn device_pixel_ratio(&self) -> f32;
  fn begin_frame(&mut self);
  fn draw_commands(&mut self, viewport: Rect, commands: Vec<PaintCommand>, surface: Color);
  fn end_frame(&mut self);
}

impl Window {
  #[deprecated(note = "The core window should not depends on shell window event.")]
  #[inline]
  /// processes native events from this native window
  pub fn processes_native_event(&self, event: WindowEvent) {
    let ratio = self.device_pixel_ratio() as f64;
    self.dispatcher.borrow_mut().dispatch(event, ratio);
  }

  /// Request switch the focus to next widget.
  pub fn request_next_focus(&self) { self.focus_mgr.borrow_mut().focus_next_widget(); }

  /// Request switch the focus to prev widget.
  pub fn request_prev_focus(&self) { self.focus_mgr.borrow_mut().focus_prev_widget(); }

  /// Return an `rxRust` Scheduler, which will guarantee all task add to the
  /// scheduler will finished before current frame finished.
  #[inline]
  pub fn frame_scheduler(&self) -> FuturesLocalScheduler { self.frame_pool.borrow().spawner() }

  /// Spawns a task that polls the given future with output `()` to completion.
  /// And guarantee wait this task will finished in current frame.
  pub fn frame_spawn(&self, f: impl Future<Output = ()> + 'static) -> Result<(), SpawnError> {
    self.frame_scheduler().spawn_local(f)
  }

  pub fn frame_tick_stream(&self) -> Subject<'static, FrameMsg, Infallible> {
    self.frame_ticker.frame_tick_stream()
  }

  pub fn inc_running_animate(&self) { self.running_animates.set(self.running_animates.get() + 1); }

  pub fn dec_running_animate(&self) { self.running_animates.set(self.running_animates.get() - 1); }

  /// Draw an image what current render tree represent.
  #[track_caller]
  pub fn draw_frame(&self) -> bool {
    self.run_frame_tasks();
    self.frame_ticker.emit(FrameMsg::NewFrame(Instant::now()));
    self.update_painter_bound();
    let draw = self.need_draw() && !self.size().is_empty();
    if draw {
      self.shell_wnd.borrow_mut().begin_frame();

      self.layout();

      self.widget_tree.borrow().draw();
      self.draw_delay_drop_widgets();

      let surface = match AppCtx::app_theme() {
        Theme::Full(theme) => theme.palette.surface(),
        Theme::Inherit(_) => unreachable!(),
      };

      let mut shell = self.shell_wnd.borrow_mut();
      let inner_size = shell.inner_size();
      let paint_cmds = self.painter.borrow_mut().finish();
      shell.draw_commands(Rect::from_size(inner_size), paint_cmds, surface);

      shell.end_frame();
    }

    AppCtx::end_frame();
    self.frame_ticker.emit(FrameMsg::Finish(Instant::now()));

    draw
  }

  pub fn layout(&self) {
    loop {
      self.run_frame_tasks();

      self
        .widget_tree
        .borrow_mut()
        .layout(self.shell_wnd.borrow().inner_size());

      if !self.widget_tree.borrow().is_dirty() {
        self.focus_mgr.borrow_mut().refresh_focus();
      }

      // we need to run frame tasks before we emit `FrameMsg::LayoutReady` to keep the
      // task and event emit order.
      if !self.widget_tree.borrow().is_dirty() {
        self.run_frame_tasks();
      }
      if !self.widget_tree.borrow().is_dirty() {
        let ready = FrameMsg::LayoutReady(Instant::now());
        self.frame_ticker.emit(ready);
      }

      if !self.widget_tree.borrow().is_dirty() {
        break;
      }
    }
  }

  pub fn update_painter_bound(&self) {
    let size = self.shell_wnd.borrow().inner_size();
    if self.painter.borrow().paint_bounds().size != size {
      let mut tree = self.widget_tree.borrow_mut();
      let root = tree.root();
      tree.mark_dirty(root);
      tree.store.remove(root);
      let mut painter = self.painter.borrow_mut();
      painter.set_bounds(Rect::from_size(size));
      painter.reset();
    }
  }

  pub fn need_draw(&self) -> bool {
    self.widget_tree.borrow().is_dirty()
      || self.running_animates.get() > 0
      // if a `pipe` widget is regenerating, need a new frame to finish it.
      || !self.regenerating_subtree.borrow().is_empty()
  }

  pub fn new(shell_wnd: Box<dyn ShellWindow>) -> Rc<Self> {
    let focus_mgr = RefCell::new(FocusManager::new());
    let widget_tree = RefCell::new(WidgetTree::default());
    let dispatcher = RefCell::new(Dispatcher::new());
    let size = shell_wnd.inner_size();
    let mut painter = Painter::new(Rect::from_size(size));
    painter.set_bounds(Rect::from_size(size));
    let window = Self {
      dispatcher,
      widget_tree,
      painter: RefCell::new(painter),
      focus_mgr,
      delay_emitter: <_>::default(),
      frame_ticker: FrameTicker::default(),
      running_animates: <_>::default(),
      frame_pool: <_>::default(),
      shell_wnd: RefCell::new(shell_wnd),
      delay_drop_widgets: <_>::default(),
      regenerating_subtree: <_>::default(),
    };
    let window = Rc::new(window);
    window.dispatcher.borrow_mut().init(Rc::downgrade(&window));
    window.focus_mgr.borrow_mut().init(Rc::downgrade(&window));
    window.widget_tree.borrow_mut().init(Rc::downgrade(&window));

    window
  }

  pub fn set_content_widget(&self, root: impl WidgetBuilder) {
    let build_ctx = BuildCtx::new(None, &self.widget_tree);
    let root = root.widget_build(&build_ctx);
    self.widget_tree.borrow_mut().set_root(root.consume())
  }

  #[inline]
  pub fn id(&self) -> WindowId { self.shell_wnd.borrow().id() }

  /// Return the current focused widget id.
  pub fn focusing(&self) -> Option<WidgetId> { self.focus_mgr.borrow().focusing() }

  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  pub fn device_pixel_ratio(&self) -> f32 { self.shell_wnd.borrow().device_pixel_ratio() }

  pub fn set_title(&self, title: &str) { self.shell_wnd.borrow_mut().set_title(title); }

  pub fn set_icon(&self, icon: &PixelImage) { self.shell_wnd.borrow_mut().set_icon(icon); }

  /// Returns the cursor icon of the window.
  pub fn get_cursor(&self) -> CursorIcon { self.shell_wnd.borrow().cursor() }

  /// Modifies the cursor icon of the window.
  pub fn set_cursor(&self, cursor: CursorIcon) { self.shell_wnd.borrow_mut().set_cursor(cursor); }

  /// Sets location of IME candidate box in window global coordinates relative
  /// to the top left.
  pub fn set_ime_cursor_area(&self, rect: &Rect) {
    self.shell_wnd.borrow_mut().set_ime_cursor_area(rect);
  }

  pub fn request_resize(&self, size: Size) { self.shell_wnd.borrow_mut().request_resize(size) }

  pub fn size(&self) -> Size { self.shell_wnd.borrow().inner_size() }

  pub fn set_min_size(&self, size: Size) { self.shell_wnd.borrow_mut().set_min_size(size); }

  pub fn shell_wnd(&self) -> &RefCell<Box<dyn ShellWindow>> { &self.shell_wnd }

  pub(crate) fn add_focus_node(&self, wid: WidgetId, auto_focus: bool, focus_type: FocusType) {
    self
      .focus_mgr
      .borrow_mut()
      .add_focus_node(wid, auto_focus, focus_type);
  }

  pub(crate) fn remove_focus_node(&self, wid: WidgetId, focus_tyep: FocusType) {
    self
      .focus_mgr
      .borrow_mut()
      .remove_focus_node(wid, focus_tyep);
  }

  pub(crate) fn add_delay_event(&self, e: DelayEvent) {
    self.delay_emitter.borrow_mut().push_back(e);
  }

  pub(crate) fn is_in_another_regenerating(&self, wid: WidgetId) -> bool {
    let regen = self.regenerating_subtree.borrow();
    if regen.is_empty() {
      return false;
    }
    let tree = self.widget_tree.borrow();
    let Some(p) = wid.parent(&tree.arena) else {
      return false;
    };
    let in_another = p.ancestors(&tree.arena).any(|p| {
      regen.get(&p).map_or(false, |to| {
        to.map_or(true, |to| wid.ancestor_of(to, &tree.arena))
      })
    });
    in_another
  }

  pub(crate) fn mark_widgets_regenerating(&self, from: WidgetId, to: Option<WidgetId>) {
    self.regenerating_subtree.borrow_mut().insert(from, to);
  }

  pub(crate) fn remove_regenerating_mark(&self, from: WidgetId) {
    self.regenerating_subtree.borrow_mut().remove(&from);
  }

  fn draw_delay_drop_widgets(&self) {
    let mut delay_widgets = self.delay_drop_widgets.borrow_mut();
    let mut painter = self.painter.borrow_mut();

    delay_widgets.retain(|(parent, wid)| {
      let tree = self.widget_tree.borrow();
      let drop_conditional = wid
        .assert_get(&self.widget_tree.borrow().arena)
        .query_most_outside(|d: &DelayDrop| d.delay_drop_until)
        .unwrap_or(true);
      let parent_dropped = parent.map_or(false, |p| {
        p.ancestors(&tree.arena).last() != Some(tree.root())
      });
      let need_drop = drop_conditional || parent_dropped;
      if need_drop {
        drop(tree);
        self.widget_tree.borrow_mut().remove_subtree(*wid);
      } else {
        let mut painter = painter.save_guard();
        if let Some(p) = parent {
          let offset = tree.store.map_to_global(Point::zero(), *p, &tree.arena);
          painter.translate(offset.x, offset.y);
        }
        let mut ctx = PaintingCtx::new(*wid, self.id(), &mut painter);
        wid.paint_subtree(&mut ctx);
      }
      !need_drop
    });
  }

  /// Immediately emit all delay events. You should not call this method only if
  /// you want to interfere with the framework event dispatch process and know
  /// what you are doing.
  fn emit_events(&self) {
    loop {
      let Some(e) = self.delay_emitter.borrow_mut().pop_front() else {
        break;
      };

      match e {
        DelayEvent::Mounted(id) => {
          let e = AllLifecycle::Mounted(LifecycleEvent { id, wnd_id: self.id() });
          self.emit::<LifecycleListener>(id, e);
        }
        DelayEvent::PerformedLayout(id) => {
          let e = AllLifecycle::PerformedLayout(LifecycleEvent { id, wnd_id: self.id() });
          self.emit::<LifecycleListener>(id, e);
        }
        DelayEvent::Disposed { id, parent } => {
          id.descendants(&self.widget_tree.borrow().arena)
            .for_each(|id| {
              let e = AllLifecycle::Disposed(LifecycleEvent { id, wnd_id: self.id() });
              self.emit::<LifecycleListener>(id, e);
            });

          let delay_drop = id
            .assert_get(&self.widget_tree.borrow().arena)
            .contain_type::<DelayDrop>();

          if delay_drop {
            self.delay_drop_widgets.borrow_mut().push((parent, id));
          } else {
            self.widget_tree.borrow_mut().remove_subtree(id);
          }
        }
        DelayEvent::Focus(id) => {
          let e = AllFocus::Focus(FocusEvent::new(id, self.id()));
          self.emit::<FocusListener>(id, e);
        }
        DelayEvent::FocusIn { bottom, up } => {
          let mut e = AllFocusBubble::FocusInCapture(FocusEvent::new(bottom, self.id()));
          self.top_down_emit::<FocusBubbleListener>(&mut e, bottom, up);
          let mut e = AllFocusBubble::FocusIn(e.into_inner());
          self.bottom_up_emit::<FocusBubbleListener>(&mut e, bottom, up);
        }
        DelayEvent::Blur(id) => {
          let e = AllFocus::Blur(FocusEvent::new(id, self.id()));
          self.emit::<FocusListener>(id, e);
        }
        DelayEvent::FocusOut { bottom, up } => {
          let mut e = AllFocusBubble::FocusOutCapture(FocusEvent::new(bottom, self.id()));
          self.top_down_emit::<FocusBubbleListener>(&mut e, bottom, up);
          let mut e = AllFocusBubble::FocusOut(e.into_inner());
          self.bottom_up_emit::<FocusBubbleListener>(&mut e, bottom, up);
        }
        DelayEvent::KeyDown { id, physical_key, key } => {
          let mut e = AllKeyboard::KeyDownCapture(KeyboardEvent::new(
            physical_key,
            key.clone(),
            id,
            self.id(),
          ));
          self.top_down_emit::<KeyboardListener>(&mut e, id, None);
          let mut e = AllKeyboard::KeyDown(e.into_inner());
          self.bottom_up_emit::<KeyboardListener>(&mut e, id, None);

          if !e.is_prevent_default() && key == VirtualKey::Named(NamedKey::Tab) {
            let pressed_shift = {
              let dispatcher = self.dispatcher.borrow();
              dispatcher.info.modifiers().contains(ModifiersState::SHIFT)
            };

            let mut focus_mgr = self.focus_mgr.borrow_mut();
            if pressed_shift {
              focus_mgr.focus_prev_widget();
            } else {
              focus_mgr.focus_next_widget();
            }
          }
        }
        DelayEvent::KeyUp { id, physical_key, key } => {
          let mut e =
            AllKeyboard::KeyUpCapture(KeyboardEvent::new(physical_key, key, id, self.id()));
          self.top_down_emit::<KeyboardListener>(&mut e, id, None);
          let mut e = AllKeyboard::KeyUp(e.into_inner());
          self.bottom_up_emit::<KeyboardListener>(&mut e, id, None);
        }
        DelayEvent::Chars { id, chars } => {
          let mut e = AllChars::CharsCapture(CharsEvent::new(chars, id, self.id()));
          self.top_down_emit::<CharsListener>(&mut e, id, None);
          let mut e = AllChars::Chars(e.into_inner());
          self.bottom_up_emit::<CharsListener>(&mut e, id, None);
        }
        DelayEvent::Wheel { id, delta_x, delta_y } => {
          let mut e = AllWheel::WheelCapture(WheelEvent::new(delta_x, delta_y, id, self.id()));
          self.top_down_emit::<WheelListener>(&mut e, id, None);
          let mut e = AllWheel::Wheel(e.into_inner());
          self.bottom_up_emit::<WheelListener>(&mut e, id, None);
        }
        DelayEvent::PointerDown(id) => {
          let mut e = AllPointer::PointerDownCapture(PointerEvent::from_mouse(id, self));
          self.top_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerDown(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
          self.focus_mgr.borrow_mut().refresh_focus();
        }
        DelayEvent::PointerMove(id) => {
          let mut e = AllPointer::PointerMoveCapture(PointerEvent::from_mouse(id, self));
          self.top_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerMove(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::PointerUp(id) => {
          let mut e = AllPointer::PointerUpCapture(PointerEvent::from_mouse(id, self));
          self.top_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerUp(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::_PointerCancel(id) => {
          let mut e = AllPointer::PointerCancel(PointerEvent::from_mouse(id, self));
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::PointerEnter { bottom, up } => {
          let mut e = AllPointer::PointerEnter(PointerEvent::from_mouse(bottom, self));
          self.top_down_emit::<PointerListener>(&mut e, bottom, up);
        }
        DelayEvent::PointerLeave { bottom, up } => {
          let mut e = AllPointer::PointerLeave(PointerEvent::from_mouse(bottom, self));
          self.bottom_up_emit::<PointerListener>(&mut e, bottom, up);
        }
        DelayEvent::Tap(wid) => {
          let mut e = AllPointer::TapCapture(PointerEvent::from_mouse(wid, self));
          self.top_down_emit::<PointerListener>(&mut e, wid, None);
          let mut e = AllPointer::Tap(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, wid, None);
        }
      }
    }
  }

  fn emit<L>(&self, id: WidgetId, mut e: L::Event)
  where
    L: EventListener + 'static,
  {
    // Safety: we only use tree to query the inner data of a node and dispatch a
    // event by it, and never read or write the node. And in the callback, there is
    // no way to mut access the inner data of node or destroy the node.
    let tree = unsafe { &*(&*self.widget_tree.borrow() as *const WidgetTree) };
    id.assert_get(&tree.arena).query_type_inside_first(|m: &L| {
      m.dispatch(&mut e);
      true
    });
  }

  fn top_down_emit<L>(&self, e: &mut L::Event, bottom: WidgetId, up: Option<WidgetId>)
  where
    L: EventListener + 'static,
    L::Event: DerefMut,
    <L::Event as Deref>::Target: std::borrow::BorrowMut<CommonEvent>,
  {
    use std::borrow::Borrow;

    let tree = self.widget_tree.borrow();
    let path = bottom
      .ancestors(&tree.arena)
      .take_while(|id| Some(*id) != up)
      .collect::<Vec<_>>();

    path.iter().rev().all(|id| {
      id.assert_get(&tree.arena)
        .query_type_outside_first(|m: &L| {
          (**e).borrow_mut().set_current_target(*id);
          m.dispatch(e);
          (**e).borrow_mut().is_propagation()
        });
      (**e).borrow().is_propagation()
    });
  }

  fn bottom_up_emit<L>(&self, e: &mut L::Event, bottom: WidgetId, up: Option<WidgetId>)
  where
    L: EventListener + 'static,
    L::Event: DerefMut,
    <L::Event as Deref>::Target: std::borrow::BorrowMut<CommonEvent>,
  {
    use std::borrow::Borrow;
    if !(**e).borrow().is_propagation() {
      return;
    }

    let tree = self.widget_tree.borrow();
    bottom
      .ancestors(&tree.arena)
      .take_while(|id| Some(*id) != up)
      .all(|id| {
        id.assert_get(&tree.arena).query_type_inside_first(|m: &L| {
          (**e).borrow_mut().set_current_target(id);
          m.dispatch(e);
          (**e).borrow_mut().is_propagation()
        });
        (**e).borrow().is_propagation()
      });
  }

  /// Run all async tasks need finished in current frame and emit all delay
  /// events.
  pub fn run_frame_tasks(&self) {
    loop {
      // wait all frame task finished.
      self.frame_pool.borrow_mut().run();
      // run all ready async tasks
      AppCtx::run_until_stalled();
      if !self.delay_emitter.borrow().is_empty() {
        self.emit_events();
      } else {
        break;
      }
    }
  }
}

/// Event that delay to emit, emit it when the window is not busy(nobody borrow
/// parts of the window).
#[derive(Debug)]
pub(crate) enum DelayEvent {
  Mounted(WidgetId),
  PerformedLayout(WidgetId),
  Disposed {
    parent: Option<WidgetId>,
    id: WidgetId,
  },
  Focus(WidgetId),
  Blur(WidgetId),
  FocusIn {
    bottom: WidgetId,
    up: Option<WidgetId>,
  },
  FocusOut {
    bottom: WidgetId,
    up: Option<WidgetId>,
  },
  KeyDown {
    id: WidgetId,
    physical_key: PhysicalKey,
    key: VirtualKey,
  },
  KeyUp {
    id: WidgetId,
    physical_key: PhysicalKey,
    key: VirtualKey,
  },
  Chars {
    id: WidgetId,
    chars: String,
  },
  Wheel {
    id: WidgetId,
    delta_x: f32,
    delta_y: f32,
  },
  PointerDown(WidgetId),
  PointerMove(WidgetId),
  PointerUp(WidgetId),
  _PointerCancel(WidgetId),
  PointerEnter {
    bottom: WidgetId,
    up: Option<WidgetId>,
  },
  PointerLeave {
    bottom: WidgetId,
    up: Option<WidgetId>,
  },
  Tap(WidgetId),
}

impl From<u64> for WindowId {
  #[inline]
  fn from(value: u64) -> Self { WindowId(value) }
}

impl From<WindowId> for u64 {
  #[inline]
  fn from(value: WindowId) -> Self { value.0 }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};
  use ribir_dev_helper::assert_layout_result_by_path;

  #[test]
  fn layout_after_wnd_resize() {
    reset_test_env!();

    let size = Size::new(100., 100.);
    let mut wnd = TestWindow::new_with_size(fn_widget! { MockBox { size: INFINITY_SIZE } }, size);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == size, });

    let new_size = Size::new(200., 200.);
    wnd.request_resize(new_size);

    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == new_size, });
  }
}
