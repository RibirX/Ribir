use crate::{
  animation::AnimateTrack,
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
  cell::RefCell,
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
  pub(crate) running_animates: Rc<RefCell<u32>>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct WindowId(u64);

pub trait ShellWindow {
  fn id(&self) -> WindowId;
  fn inner_size(&self) -> Size;
  fn outer_size(&self) -> Size;
  fn set_ime_pos(&mut self, pos: Point);
  fn set_size(&mut self, size: Size);
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
    self.emit_events();
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

  pub fn animate_track(&self) -> AnimateTrack {
    AnimateTrack {
      actived: false,
      actived_cnt: self.running_animates.clone(),
    }
  }

  /// Draw an image what current render tree represent.
  #[track_caller]
  pub fn draw_frame(&self) {
    self.emit_events();

    if !self.need_draw() || self.size().is_empty() {
      return;
    }

    self.frame_ticker.emit(FrameMsg::NewFrame(Instant::now()));

    self.shell_wnd.borrow_mut().begin_frame();

    loop {
      self.layout();
      self.emit_events();

      // wait all frame task finished.
      self.frame_pool.borrow_mut().run();

      if !self.widget_tree.borrow().is_dirty() {
        self.focus_mgr.borrow_mut().refresh_focus();
        self.emit_events();

        // focus refresh and event emit may cause widget tree dirty again.
        if !self.widget_tree.borrow().is_dirty() {
          break;
        }
      }
    }

    self.widget_tree.borrow().draw();

    let surface = match AppCtx::app_theme() {
      Theme::Full(theme) => theme.palette.surface(),
      Theme::Inherit(_) => unreachable!(),
    };

    let mut shell = self.shell_wnd.borrow_mut();
    let inner_size = shell.inner_size();
    let paint_cmds = self.painter.borrow_mut().finish();
    shell.draw_commands(Rect::from_size(inner_size), paint_cmds, surface);

    shell.end_frame();
    self.frame_ticker.emit(FrameMsg::Finish(Instant::now()));
    AppCtx::end_frame();
  }

  pub fn layout(&self) {
    self
      .widget_tree
      .borrow_mut()
      .layout(self.shell_wnd.borrow().inner_size());

    self
      .frame_ticker
      .emit(FrameMsg::LayoutReady(Instant::now()));
  }

  pub fn need_draw(&self) -> bool {
    self.widget_tree.borrow().is_dirty() || *self.running_animates.borrow() > 0
  }

  pub fn new(root: Widget, shell_wnd: Box<dyn ShellWindow>) -> Rc<Self> {
    let focus_mgr = RefCell::new(FocusManager::new());
    let widget_tree = RefCell::new(WidgetTree::new());
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
      running_animates: Rc::new(RefCell::new(0)),
      frame_pool: RefCell::new(<_>::default()),
      shell_wnd: RefCell::new(shell_wnd),
    };
    let window = Rc::new(window);
    window.dispatcher.borrow_mut().init(Rc::downgrade(&window));
    window.focus_mgr.borrow_mut().init(Rc::downgrade(&window));
    window
      .widget_tree
      .borrow_mut()
      .init(root, Rc::downgrade(&window));

    window
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
  pub fn set_ime_pos(&self, pos: Point) { self.shell_wnd.borrow_mut().set_ime_pos(pos); }

  pub fn set_size(&self, size: Size) { self.shell_wnd.borrow_mut().set_size(size); }

  pub fn size(&self) -> Size { self.shell_wnd.borrow().inner_size() }

  pub fn set_min_size(&self, size: Size) { self.shell_wnd.borrow_mut().set_min_size(size); }

  pub fn on_wnd_resize_event(&self, size: Size) {
    let mut tree = self.widget_tree.borrow_mut();
    let root = tree.root();
    tree.mark_dirty(root);
    tree.store.remove(root);
    let mut painter = self.painter.borrow_mut();
    painter.set_bounds(Rect::from_size(size));
    painter.reset();
  }

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

  /// Immediately emit all delay events. You should not call this method only if
  /// you want to interfere with the framework event dispatch process and know
  /// what you are doing.
  pub fn emit_events(&self) {
    loop {
      let Some(e) = self.delay_emitter.borrow_mut().pop_front() else{ break};

      match e {
        DelayEvent::Mounted(id) => {
          let e = AllLifecycle::Mounted(LifecycleEvent { id, wnd: self });
          self.emit::<LifecycleListener>(id, e);
        }
        DelayEvent::PerformedLayout(id) => {
          let e = AllLifecycle::PerformedLayout(LifecycleEvent { id, wnd: self });
          self.emit::<LifecycleListener>(id, e);
        }
        DelayEvent::Disposed { id, delay_drop } => {
          id.descendants(&self.widget_tree.borrow().arena)
            .for_each(|id| {
              let e = AllLifecycle::Disposed(LifecycleEvent { id, wnd: self });
              self.emit::<LifecycleListener>(id, e);
            });

          if !delay_drop {
            self.widget_tree.borrow_mut().remove_subtree(id);
          }
        }
        DelayEvent::Focus(id) => {
          let e = AllFocus::Focus(FocusEvent::new(id, self));
          self.emit::<FocusListener>(id, e);
        }
        DelayEvent::FocusIn { bottom, up } => {
          let mut e = AllFocusBubble::FocusInCapture(FocusEvent::new(bottom, self));
          self.bottom_down_emit::<FocusBubbleListener>(&mut e, bottom, up);
          let mut e = AllFocusBubble::FocusIn(e.into_inner());
          self.bottom_up_emit::<FocusBubbleListener>(&mut e, bottom, up);
        }
        DelayEvent::Blur(id) => {
          let e = AllFocus::Blur(FocusEvent::new(id, self));
          self.emit::<FocusListener>(id, e);
        }
        DelayEvent::FocusOut { bottom, up } => {
          let mut e = AllFocusBubble::FocusOutCapture(FocusEvent::new(bottom, self));
          self.bottom_down_emit::<FocusBubbleListener>(&mut e, bottom, up);
          let mut e = AllFocusBubble::FocusOut(e.into_inner());
          self.bottom_up_emit::<FocusBubbleListener>(&mut e, bottom, up);
        }
        DelayEvent::KeyDown { id, scancode, key } => {
          let mut e = AllKeyboard::KeyDownCapture(KeyboardEvent::new(scancode, key, id, self));
          self.bottom_down_emit::<KeyboardListener>(&mut e, id, None);
          let mut e = AllKeyboard::KeyDown(e.into_inner());
          self.bottom_up_emit::<KeyboardListener>(&mut e, id, None);

          if !e.is_prevent_default() && key == VirtualKeyCode::Tab {
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
        DelayEvent::KeyUp { id, scancode, key } => {
          let mut e = AllKeyboard::KeyUpCapture(KeyboardEvent::new(scancode, key, id, self));
          self.bottom_down_emit::<KeyboardListener>(&mut e, id, None);
          let mut e = AllKeyboard::KeyUp(e.into_inner());
          self.bottom_up_emit::<KeyboardListener>(&mut e, id, None);
        }
        DelayEvent::Chars { id, chars } => {
          let mut e = AllChars::CharsCapture(CharsEvent::new(chars, id, self));
          self.bottom_down_emit::<CharsListener>(&mut e, id, None);
          let mut e = AllChars::Chars(e.into_inner());
          self.bottom_up_emit::<CharsListener>(&mut e, id, None);
        }
        DelayEvent::Wheel { id, delta_x, delta_y } => {
          let mut e = AllWheel::WheelCapture(WheelEvent::new(delta_x, delta_y, id, self));
          self.bottom_down_emit::<WheelListener>(&mut e, id, None);
          let mut e = AllWheel::Wheel(e.into_inner());
          self.bottom_up_emit::<WheelListener>(&mut e, id, None);
        }
        DelayEvent::PointerDown(id) => {
          let mut e = AllPointer::PointerDownCapture(PointerEvent::from_mouse(id, self));
          self.bottom_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerDown(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
          self.focus_mgr.borrow_mut().refresh_focus();
        }
        DelayEvent::PointerMove(id) => {
          let mut e = AllPointer::PointerMoveCapture(PointerEvent::from_mouse(id, self));
          self.bottom_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerMove(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::PointerUp(id) => {
          let mut e = AllPointer::PointerUpCapture(PointerEvent::from_mouse(id, self));
          self.bottom_down_emit::<PointerListener>(&mut e, id, None);
          let mut e = AllPointer::PointerUp(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::_PointerCancel(id) => {
          let mut e = AllPointer::PointerCancel(PointerEvent::from_mouse(id, self));
          self.bottom_up_emit::<PointerListener>(&mut e, id, None);
        }
        DelayEvent::PointerEnter { bottom, up } => {
          let mut e = AllPointer::PointerEnter(PointerEvent::from_mouse(bottom, self));
          self.bottom_down_emit::<PointerListener>(&mut e, bottom, up);
        }
        DelayEvent::PointerLeave { bottom, up } => {
          let mut e = AllPointer::PointerLeave(PointerEvent::from_mouse(bottom, self));
          self.bottom_up_emit::<PointerListener>(&mut e, bottom, up);
        }
        DelayEvent::Tap(wid) => {
          let mut e = AllPointer::TapCapture(PointerEvent::from_mouse(wid, self));
          self.bottom_down_emit::<PointerListener>(&mut e, wid, None);
          let mut e = AllPointer::Tap(e.into_inner());
          self.bottom_up_emit::<PointerListener>(&mut e, wid, None);
        }
      }
    }
  }

  fn emit<L>(&self, id: WidgetId, mut e: L::Event<'_>)
  where
    L: EventListener + 'static,
  {
    // Safety: we only use tree to query the inner data of a node and dispatch a
    // event by it, and never read or write the node. And in the callback, there is
    // no way to mut access the inner data of node or destroy the node.
    let tree = unsafe { &*(&*self.widget_tree.borrow() as *const WidgetTree) };
    id.assert_get(&tree.arena).query_all_type(
      |m: &L| {
        m.dispatch(&mut e);
        true
      },
      QueryOrder::InnerFirst,
    );
  }

  fn bottom_down_emit<'a, L>(&self, e: &mut L::Event<'a>, bottom: WidgetId, up: Option<WidgetId>)
  where
    L: EventListener + 'static,
    L::Event<'a>: DerefMut,
    <L::Event<'a> as Deref>::Target: std::borrow::BorrowMut<CommonEvent<'a>>,
  {
    use std::borrow::Borrow;

    let tree = self.widget_tree.borrow();
    let path = bottom
      .ancestors(&tree.arena)
      .take_while(|id| Some(*id) != up)
      .collect::<Vec<_>>();

    path.iter().rev().all(|id| {
      id.assert_get(&tree.arena).query_all_type(
        |m: &L| {
          (**e).borrow_mut().set_current_target(*id);
          m.dispatch(e);
          (**e).borrow_mut().is_propagation()
        },
        QueryOrder::OutsideFirst,
      );
      (**e).borrow().is_propagation()
    });
  }

  fn bottom_up_emit<'a, L>(&self, e: &mut L::Event<'a>, bottom: WidgetId, up: Option<WidgetId>)
  where
    L: EventListener + 'static,
    L::Event<'a>: DerefMut,
    <L::Event<'a> as Deref>::Target: std::borrow::BorrowMut<CommonEvent<'a>>,
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
        id.assert_get(&tree.arena).query_all_type(
          |m: &L| {
            (**e).borrow_mut().set_current_target(id);
            m.dispatch(e);
            (**e).borrow_mut().is_propagation()
          },
          QueryOrder::InnerFirst,
        );
        (**e).borrow().is_propagation()
      });
  }
}

/// Event that delay to emit, emit it when the window is not busy(nobody borrow
/// parts of the window).
#[derive(Debug)]
pub(crate) enum DelayEvent {
  Mounted(WidgetId),
  PerformedLayout(WidgetId),
  Disposed {
    id: WidgetId,
    delay_drop: bool,
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
    scancode: ScanCode,
    key: VirtualKeyCode,
  },
  KeyUp {
    id: WidgetId,
    scancode: ScanCode,
    key: VirtualKeyCode,
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
  use crate::test_helper::*;
  use ribir_dev_helper::assert_layout_result_by_path;

  #[test]
  fn layout_after_wnd_resize() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = widget! {
       MockBox { size: INFINITY_SIZE }
    };
    let size = Size::new(100., 100.);
    let mut wnd = TestWindow::new_with_size(w, size);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == size, });

    let new_size = Size::new(200., 200.);
    wnd.set_size(new_size);
    // not have a shell window, trigger the resize manually.
    wnd.on_wnd_resize_event(new_size);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == new_size, });
  }
}
