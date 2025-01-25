use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  convert::Infallible,
  ptr::NonNull,
};

use futures::{Future, task::LocalSpawnExt};
use ribir_algo::Sc;
use smallvec::SmallVec;
use widget_id::TrackId;
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};
pub use winit::window::CursorIcon;

use crate::{
  events::{
    dispatcher::Dispatcher,
    focus_mgr::{FocusManager, FocusType},
  },
  prelude::*,
  ticker::{FrameMsg, FrameTicker},
};

/// Window is the root to represent.
///
/// We use `RefCell` to wrap every field of `Window` to make sure we can split
/// borrow the fields in runtime. So we can pass `Window` to user when the
/// framework borrower  one of the fields. e.g. `dispatcher` is borrowed when
/// dispatch the event, but user may access the `Window` to change the title in
/// event callback.
pub struct Window {
  pub(crate) tree: NonNull<WidgetTree>,
  pub(crate) painter: RefCell<Painter>,
  pub(crate) dispatcher: RefCell<Dispatcher>,
  pub(crate) frame_ticker: FrameTicker,
  pub(crate) focus_mgr: RefCell<FocusManager>,
  pub(crate) running_animates: Sc<Cell<u32>>,
  pre_edit: RefCell<Option<String>>,
  /// This vector store the task to emit events. When perform layout, dispatch
  /// event and so on, some part of window may be already mutable borrowed and
  /// the user event callback may also query borrow that part, so we can't emit
  /// event immediately. So we store the event emitter in this vector,
  /// and emit them after all borrow finished.
  pub(crate) delay_emitter: RefCell<VecDeque<DelayEvent>>,
  /// A task pool use to process `Future` or `rxRust` task, and will block until
  /// all task finished before current frame end.
  frame_pool: RefCell<FuturesLocalSchedulerPool>,
  /// A priority queue of tasks. So that tasks with lower priority value will be
  /// executed first.
  priority_task_queue: PriorityTaskQueue,
  shell_wnd: RefCell<Box<dyn ShellWindow>>,
  /// A vector store the widget id pair of (parent, child). The child need to
  /// drop after its `KeepAlive::keep_alive` be false or its parent
  /// is dropped.
  ///
  /// This widgets it's detached from its parent, but still need to paint.
  pub(crate) delay_drop_widgets: RefCell<Vec<(Option<WidgetId>, TrackId)>>,

  flags: Cell<WindowFlags>,
}

bitflags! {
  #[derive(Clone, Copy)]
  #[doc="A set of flags to control the window behavior."]
  pub struct WindowFlags: u32 {
    #[doc="If this window enables animation, set this flag to true to \
    activate all animations; if this flag is not marked, all animations\
    will not run."]
    const ANIMATIONS = 1 << 0;
    const DEFAULT = Self::ANIMATIONS.bits();
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct WindowId(u64);

pub trait ShellWindow {
  fn id(&self) -> WindowId;
  fn inner_size(&self) -> Size;
  fn outer_size(&self) -> Size;
  fn set_ime_cursor_area(&mut self, rect: &Rect);
  fn set_ime_allowed(&mut self, allowed: bool);

  fn request_resize(&mut self, size: Size);
  fn on_resize(&mut self, size: Size);
  fn set_min_size(&mut self, size: Size);
  fn cursor(&self) -> CursorIcon;
  fn set_cursor(&mut self, cursor: CursorIcon);
  fn set_title(&mut self, str: &str);
  fn set_icon(&mut self, icon: &PixelImage);
  fn is_visible(&self) -> Option<bool>;
  fn set_visible(&mut self, visible: bool);
  fn is_resizable(&self) -> bool;
  fn set_resizable(&mut self, resizable: bool);
  fn is_minimized(&self) -> bool;
  fn set_minimized(&mut self, minimized: bool);
  fn focus_window(&mut self);
  fn set_decorations(&mut self, decorations: bool);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  fn device_pixel_ratio(&self) -> f32;
  fn begin_frame(&mut self, surface_color: Color);
  fn draw_commands(&mut self, viewport: Rect, commands: &[PaintCommand]);
  fn end_frame(&mut self);
}

impl Window {
  #[deprecated(note = "The core window should not depends on shell window event.")]
  #[inline]
  /// processes native events from this native window
  pub fn processes_native_event(&self, event: WindowEvent) {
    let ratio = self.device_pixel_ratio() as f64;
    self
      .dispatcher
      .borrow_mut()
      .dispatch(event, ratio);
  }

  pub fn processes_keyboard_event(
    &self, physical_key: PhysicalKey, key: VirtualKey, is_repeat: bool, location: KeyLocation,
    state: ElementState,
  ) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_keyboard_input(physical_key, key, is_repeat, location, state);
  }

  pub fn processes_receive_chars(&self, chars: String) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_receive_chars(chars)
  }

  pub fn processes_ime_pre_edit(&self, ime: ImePreEdit) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_ime_pre_edit(ime)
  }

  pub fn process_mouse_input(&self, device_id: DeviceId, state: ElementState, button: MouseButton) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_mouse_input(device_id, state, button);
  }

  /// Request switch the focus to next widget.
  pub fn request_next_focus(&self) {
    self
      .focus_mgr
      .borrow_mut()
      .focus_next_widget(self.tree());
  }

  /// Request switch the focus to prev widget.
  pub fn request_prev_focus(&self) {
    self
      .focus_mgr
      .borrow_mut()
      .focus_prev_widget(self.tree());
  }

  /// Execute the callback when the next frame begins.
  pub fn once_next_frame(&self, f: impl FnOnce() + 'static) {
    self.once_on_lifecycle(f, |msg| matches!(msg, FrameMsg::NewFrame(_)))
  }

  /// Execute the callback when the current frame finished.
  pub fn once_frame_finished(&self, f: impl FnOnce() + 'static) {
    self.once_on_lifecycle(f, |msg| matches!(msg, FrameMsg::Finish(_)))
  }

  /// Execute the callback before the next layout begins.
  pub fn once_before_layout(&self, f: impl FnOnce() + 'static) {
    self.once_on_lifecycle(f, |msg| matches!(msg, FrameMsg::BeforeLayout(_)))
  }

  /// Execute the callback when the layout is ready.
  pub fn once_layout_ready(&self, f: impl FnOnce() + 'static) {
    self.once_on_lifecycle(f, |msg| matches!(msg, FrameMsg::LayoutReady(_)))
  }

  /// Return an `rxRust` Scheduler, which will guarantee all task add to the
  /// scheduler will finished before current frame finished.
  #[inline]
  pub fn frame_scheduler(&self) -> FuturesLocalScheduler { self.frame_pool.borrow().spawner() }

  /// Spawns a task that polls the given future with output `()` to completion.
  /// And guarantee wait this task will finished in current frame.
  pub fn frame_spawn(&self, f: impl Future<Output = ()> + 'static) -> Result<(), SpawnError> {
    self.frame_scheduler().spawn_local(f)
  }

  pub fn priority_task_queue(&self) -> &PriorityTaskQueue { &self.priority_task_queue }

  pub fn frame_tick_stream(&self) -> Subject<'static, FrameMsg, Infallible> {
    self.frame_ticker.clone()
  }

  pub fn inc_running_animate(&self) {
    self
      .running_animates
      .set(self.running_animates.get() + 1);
  }

  pub fn dec_running_animate(&self) {
    self
      .running_animates
      .set(self.running_animates.get() - 1);
  }

  /// Draw an image what current render tree represent.
  #[track_caller]
  pub fn draw_frame(&self) -> bool {
    AppCtx::run_until_stalled();
    let mut ticker = self.frame_ticker.clone();
    ticker.next(FrameMsg::NewFrame(Instant::now()));
    self.run_frame_tasks();

    self.update_painter_viewport();
    let draw = self.need_draw() && !self.size().is_empty();
    if draw {
      let root = self.tree().root();

      let surface = {
        let _guard = BuildCtx::init_for(root, self.tree);
        Palette::of(BuildCtx::get()).surface()
      };
      self.shell_wnd.borrow_mut().begin_frame(surface);

      ticker.next(FrameMsg::BeforeLayout(Instant::now()));
      self.layout();

      self.tree().draw();
      self.draw_delay_drop_widgets();

      let mut shell = self.shell_wnd.borrow_mut();
      let inner_size = shell.inner_size();
      let mut painter = self.painter.borrow_mut();
      shell.draw_commands(Rect::from_size(inner_size), &painter.finish());

      shell.end_frame();
    }

    AppCtx::end_frame();
    ticker.next(FrameMsg::Finish(Instant::now()));
    ticker.retain();

    draw
  }

  pub fn layout(&self) {
    loop {
      self.run_frame_tasks();

      let tree = self.tree_mut();
      tree.layout(self.shell_wnd.borrow().inner_size());
      self.run_frame_tasks();

      if !tree.is_dirty() {
        self.focus_mgr.borrow_mut().refresh_focus(tree);
        self.run_frame_tasks();
      }

      if !tree.is_dirty() {
        let ready = FrameMsg::LayoutReady(Instant::now());
        self.frame_ticker.clone().next(ready);
        self.run_frame_tasks();
      }

      if !tree.is_dirty() {
        break;
      }
    }
  }

  pub fn update_painter_viewport(&self) {
    let size = self.shell_wnd.borrow().inner_size();
    if self.painter.borrow().viewport().size != size {
      let tree = self.tree_mut();
      let root = tree.root();
      tree.dirty_marker().mark(root, DirtyPhase::Layout);
      tree.store.remove(root);
      let mut painter = self.painter.borrow_mut();
      painter.set_viewport(Rect::from_size(size));
      painter.reset();
    }
  }

  pub fn need_draw(&self) -> bool { self.tree().is_dirty() || self.running_animates.get() > 0 }

  pub fn new(shell_wnd: Box<dyn ShellWindow>) -> Sc<Self> {
    let wnd_id = shell_wnd.id();
    let focus_mgr = RefCell::new(FocusManager::new(wnd_id));
    let tree = Box::new(WidgetTree::new(wnd_id));
    let dispatcher = RefCell::new(Dispatcher::new(wnd_id));
    let size = shell_wnd.inner_size();
    let painter = Painter::new(Rect::from_size(size));
    let window = Self {
      tree: NonNull::new(Box::into_raw(tree)).unwrap(),
      dispatcher,
      painter: RefCell::new(painter),
      focus_mgr,
      delay_emitter: <_>::default(),
      frame_ticker: FrameTicker::default(),
      running_animates: <_>::default(),
      frame_pool: <_>::default(),
      priority_task_queue: PriorityTaskQueue::default(),
      shell_wnd: RefCell::new(shell_wnd),
      delay_drop_widgets: <_>::default(),
      flags: Cell::new(WindowFlags::DEFAULT),
      pre_edit: <_>::default(),
    };

    Sc::new(window)
  }

  pub fn init(&self, content: GenWidget) {
    let root = self.tree_mut().init(self, content);
    let _guard = BuildCtx::init_for(root, self.tree);
    let ctx = BuildCtx::get();
    let brush = Palette::of(ctx).on_surface_variant();
    self
      .painter
      .borrow_mut()
      .set_init_state(brush.into());
  }

  #[inline]
  pub fn id(&self) -> WindowId { self.shell_wnd.borrow().id() }

  pub fn shell_wnd(&self) -> &RefCell<Box<dyn ShellWindow>> { &self.shell_wnd }

  pub fn flags(&self) -> WindowFlags { self.flags.get() }

  pub fn set_flags(&self, flags: WindowFlags) { self.flags.set(flags) }

  pub fn bubble_custom_event<E: 'static>(&self, from: WidgetId, e: E) {
    self.add_delay_event(DelayEvent::BubbleCustomEvent { from, data: Box::new(e) as Box<dyn Any> });
  }

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

  fn draw_delay_drop_widgets(&self) {
    let mut painter = self.painter.borrow_mut();

    self
      .delay_drop_widgets
      .borrow_mut()
      .retain(|(parent, wid)| {
        let wid = wid.get().unwrap();
        let tree = self.tree_mut();
        let drop_conditional = wid
          .query_ref::<KeepAlive>(tree)
          .is_none_or(|d| !d.keep_alive);
        let parent_dropped = parent
          .as_ref()
          .is_some_and(|p| p.ancestors(tree).any(|w| w.is_dropped(tree)));
        let need_drop = drop_conditional || parent_dropped;
        if need_drop {
          tree.remove_subtree(wid);
        }
        !need_drop
      });
    self
      .delay_drop_widgets
      .borrow()
      .iter()
      .for_each(|(parent, wid)| {
        if let Some(wid) = wid.get() {
          let tree = self.tree();
          let mut painter = painter.save_guard();
          if let Some(p) = parent {
            let offset = tree.map_to_global(Point::zero(), *p);
            painter.translate(offset.x, offset.y);
          }

          wid.paint_subtree(tree, &mut painter);
        }
      });
  }

  fn run_priority_tasks(&self) {
    while let Some((task, _)) = self.priority_task_queue.pop() {
      // `pipe` used priority task queue to update the subtree, we need to force
      // execute the async task and emit events in order. For example, `pipe1`
      // run first and remove a subtree, then `pipe2` run. We need to make sure
      // when `pipe2` run, the subtree is really removed.
      self.emit_events();
      task.run();
    }
  }
  /// Immediately emit all delay events. You should not call this method only if
  /// you want to interfere with the framework event dispatch process and know
  /// what you are doing.
  pub fn emit_events(&self) {
    loop {
      let Some(e) = self.delay_emitter.borrow_mut().pop_front() else {
        break;
      };

      match e {
        DelayEvent::Mounted(id) => {
          let mut e = Event::Mounted(LifecycleEvent::new(id, self.tree));
          self.emit(id, &mut e);
        }
        DelayEvent::PerformedLayout(id) => {
          let mut e = Event::PerformedLayout(LifecycleEvent::new(id, self.tree));
          self.emit(id, &mut e);
        }
        DelayEvent::Disposed { id, parent } => {
          id.descendants(self.tree())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .for_each(|id| {
              if Some(id) == self.focusing() {
                self.focus_mgr.borrow_mut().blur_on_dispose();
              }
              let mut e = Event::Disposed(LifecycleEvent::new(id, self.tree));
              self.emit(id, &mut e);
            });

          let keep_alive_id = id
            .query_ref::<KeepAlive>(self.tree())
            .filter(|d| d.keep_alive)
            .and_then(|d| d.track_id());

          if let Some(keep_alive_id) = keep_alive_id {
            self
              .delay_drop_widgets
              .borrow_mut()
              .push((parent, keep_alive_id));
          } else {
            self.add_delay_event(DelayEvent::RemoveSubtree(id));
          }
        }
        DelayEvent::RemoveSubtree(id) => {
          self.tree_mut().remove_subtree(id);
        }
        DelayEvent::Focus(id) => {
          let mut e = Event::Focus(FocusEvent::new(id, self.tree));
          self.emit(id, &mut e);
        }
        DelayEvent::FocusIn { bottom, up } => {
          let top = up.unwrap_or_else(|| self.tree().root());
          self.top_down_emit(&mut Event::FocusInCapture(FocusEvent::new(top, self.tree)), bottom);
          self.bottom_up_emit(&mut Event::FocusIn(FocusEvent::new(bottom, self.tree)), up);
        }
        DelayEvent::Blur(id) => {
          let mut e = Event::Blur(FocusEvent::new(id, self.tree));
          self.emit(id, &mut e);
        }
        DelayEvent::FocusOut { bottom, up } => {
          let top = up.unwrap_or_else(|| self.tree().root());
          self.top_down_emit(&mut Event::FocusOutCapture(FocusEvent::new(top, self.tree)), bottom);
          self.bottom_up_emit(&mut Event::FocusOut(FocusEvent::new(bottom, self.tree)), up);
        }
        DelayEvent::KeyBoard { id, physical_key, key, is_repeat, location, state } => {
          let root = self.tree().root();
          let event =
            KeyboardEvent::new(self, root, physical_key, key.clone(), is_repeat, location);
          let mut event = match state {
            ElementState::Pressed => Event::KeyDownCapture(event),
            ElementState::Released => Event::KeyUpCapture(event),
          };
          self.top_down_emit(&mut event, id);
          drop(event);
          let event = KeyboardEvent::new(self, id, physical_key, key, is_repeat, location);
          let mut event = match state {
            ElementState::Pressed => Event::KeyDown(event),
            ElementState::Released => Event::KeyUp(event),
          };
          self.bottom_up_emit(&mut event, None);
          if let Event::KeyDown(e) = event {
            if !e.is_prevent_default() && *e.key() == VirtualKey::Named(NamedKey::Tab) {
              self.add_delay_event(DelayEvent::TabFocusMove);
            }
          }
        }
        DelayEvent::TabFocusMove => {
          let pressed_shift = {
            let dispatcher = self.dispatcher.borrow();
            dispatcher
              .info
              .modifiers()
              .contains(ModifiersState::SHIFT)
          };

          let mut focus_mgr = self.focus_mgr.borrow_mut();
          if pressed_shift {
            focus_mgr.focus_prev_widget(self.tree());
          } else {
            focus_mgr.focus_next_widget(self.tree());
          }
        }

        DelayEvent::Chars { id, chars } => {
          let event = CharsEvent::new(chars.clone(), self.tree().root(), self);
          self.top_down_emit(&mut Event::CharsCapture(event), id);
          self.bottom_up_emit(&mut Event::Chars(CharsEvent::new(chars, id, self)), None);
        }
        DelayEvent::Wheel { id, delta_x, delta_y } => {
          let event = WheelEvent::new(delta_x, delta_y, self.tree().root(), self);
          self.top_down_emit(&mut Event::WheelCapture(event), id);
          self.bottom_up_emit(&mut Event::Wheel(WheelEvent::new(delta_x, delta_y, id, self)), None);
        }
        DelayEvent::PointerDown(id) => {
          let root = self.tree().root();
          let event = PointerEvent::from_mouse(root, self);
          self.top_down_emit(&mut Event::PointerDownCapture(event), id);
          self.bottom_up_emit(&mut Event::PointerDown(PointerEvent::from_mouse(id, self)), None);
          self
            .focus_mgr
            .borrow_mut()
            .refresh_focus(self.tree());
        }
        DelayEvent::PointerMove(id) => {
          let event = PointerEvent::from_mouse(self.tree().root(), self);
          self.top_down_emit(&mut Event::PointerMoveCapture(event), id);
          self.bottom_up_emit(&mut Event::PointerMove(PointerEvent::from_mouse(id, self)), None);
        }
        DelayEvent::PointerUp(id) => {
          let event = PointerEvent::from_mouse(self.tree().root(), self);
          self.top_down_emit(&mut Event::PointerUpCapture(event), id);
          let event = PointerEvent::from_mouse(id, self);
          self.bottom_up_emit(&mut Event::PointerUp(event), None);
        }
        DelayEvent::_PointerCancel(id) => {
          let event = PointerEvent::from_mouse(self.tree().root(), self);
          self.top_down_emit(&mut Event::PointerCancelCapture(event), id);
          let event = PointerEvent::from_mouse(id, self);
          self.bottom_up_emit(&mut Event::PointerCancel(event), None);
        }
        DelayEvent::PointerEnter { bottom, up } => {
          let top = up.unwrap_or_else(|| self.tree().root());
          self.top_down_emit(&mut Event::PointerEnter(PointerEvent::from_mouse(top, self)), bottom);
        }
        DelayEvent::PointerLeave { bottom, up } => {
          self.bottom_up_emit(&mut Event::PointerLeave(PointerEvent::from_mouse(bottom, self)), up);
        }
        DelayEvent::Tap(wid) => {
          let event = PointerEvent::from_mouse(self.tree().root(), self);
          self.top_down_emit(&mut Event::TapCapture(event), wid);
          let event = PointerEvent::from_mouse(wid, self);
          self.bottom_up_emit(&mut Event::Tap(event), None);
        }
        DelayEvent::ImePreEdit { wid, pre_edit } => {
          let root = self.tree().root();
          let ime_event = ImePreEditEvent::new(pre_edit.clone(), root, self);
          self.top_down_emit(&mut Event::ImePreEditCapture(ime_event), wid);
          let ime_event = ImePreEditEvent::new(pre_edit, wid, self);
          self.bottom_up_emit(&mut Event::ImePreEdit(ime_event), None);
        }
        DelayEvent::GrabPointerDown(wid) => {
          let mut e = Event::PointerDown(PointerEvent::from_mouse(wid, self));
          self.emit(wid, &mut e);
        }
        DelayEvent::GrabPointerMove(wid) => {
          let mut e = Event::PointerMove(PointerEvent::from_mouse(wid, self));
          self.emit(wid, &mut e);
        }
        DelayEvent::GrabPointerUp(wid) => {
          let mut e = Event::PointerUp(PointerEvent::from_mouse(wid, self));
          self.emit(wid, &mut e);
        }
        DelayEvent::BubbleCustomEvent { from: id, data } => {
          let mut e = Event::CustomEvent(new_custom_event(CommonEvent::new(id, self.tree), data));
          self.bottom_up_emit(&mut e, None);
        }
      }
    }
  }

  fn emit(&self, id: WidgetId, e: &mut Event) {
    id.query_all_iter::<MixBuiltin>(self.tree())
      .for_each(|m| {
        if m.contain_flag(e.flags()) {
          m.dispatch(e);
        }
      })
  }

  fn top_down_emit(&self, e: &mut Event, bottom: WidgetId) {
    let tree = self.tree();
    let path = bottom
      .ancestors(tree)
      .take_while(|id| id != &e.target())
      .collect::<Vec<_>>();

    let mut buffer = SmallVec::new();
    path.iter().rev().all(|id| {
      e.capture_to_child(*id, &mut buffer);
      id.query_all_iter::<MixBuiltin>(tree)
        .rev()
        .all(|m| {
          if m.contain_flag(e.flags()) {
            m.dispatch(e);
          }
          e.is_propagation()
        })
    });
  }

  fn bottom_up_emit(&self, e: &mut Event, up: Option<WidgetId>) {
    if !e.is_propagation() {
      return;
    }

    let tree = self.tree();
    e.target()
      .ancestors(tree)
      .take_while(|id| Some(*id) != up)
      .all(|id| {
        e.bubble_to_parent(id);
        id.query_all_iter::<MixBuiltin>(tree).all(|m| {
          if m.contain_flag(e.flags()) {
            m.dispatch(e);
          }
          e.is_propagation()
        })
      });
  }

  /// Run all async tasks need finished in current frame and emit all delay
  /// events.
  pub fn run_frame_tasks(&self) {
    loop {
      self.frame_pool.borrow_mut().run();

      if self.delay_emitter.borrow().is_empty()
        && self.priority_task_queue.is_empty()
        && AppCtx::run_until_stalled() == 0
      {
        break;
      }

      self.run_priority_tasks();
      self.emit_events();
    }
  }

  pub fn map_to_global(&self, point: Point, id: WidgetId) -> Point {
    self.tree().map_to_global(point, id)
  }

  pub fn map_from_global(&self, point: Point, id: WidgetId) -> Point {
    self.tree().map_from_global(point, id)
  }

  pub fn widget_size(&self, id: WidgetId) -> Option<Size> { self.tree().store.layout_box_size(id) }

  pub fn widget_pos(&self, id: WidgetId) -> Option<Point> { self.tree().store.layout_box_pos(id) }

  pub(crate) fn tree(&self) -> &WidgetTree {
    // Safety: Please refer to the comments in `WidgetTree::tree_mut` for more
    // information.
    unsafe { self.tree.as_ref() }
  }

  #[allow(clippy::mut_from_ref)]
  pub(crate) fn tree_mut(&self) -> &mut WidgetTree {
    let mut tree = self.tree;
    // Safety:
    // The widget tree is solely utilized for building, layout, and painting, which
    // all follow a downward flow within the tree. Therefore, even if numerous
    // mutable references exist, they only modify distinct parts of the tree.
    unsafe { tree.as_mut() }
  }
}

/// Window attributes configuration.
impl Window {
  /// Return the current focused widget id.
  pub fn focusing(&self) -> Option<WidgetId> { self.focus_mgr.borrow().focusing() }

  /// The device pixel ratio of Window interface returns the ratio of the
  /// resolution in physical pixels to the logic pixels for the current display
  /// device.
  pub fn device_pixel_ratio(&self) -> f32 { self.shell_wnd.borrow().device_pixel_ratio() }

  pub fn set_title(&self, title: &str) -> &Self {
    self.shell_wnd.borrow_mut().set_title(title);
    self
  }

  pub fn set_icon(&self, icon: &PixelImage) -> &Self {
    self.shell_wnd.borrow_mut().set_icon(icon);
    self
  }

  /// Returns the cursor icon of the window.
  pub fn get_cursor(&self) -> CursorIcon { self.shell_wnd.borrow().cursor() }

  /// Modifies the cursor icon of the window.
  pub fn set_cursor(&self, cursor: CursorIcon) -> &Self {
    self.shell_wnd.borrow_mut().set_cursor(cursor);
    self
  }

  /// Sets location of IME candidate box in window global coordinates relative
  /// to the top left.
  pub fn set_ime_cursor_area(&self, rect: &Rect) -> &Self {
    self
      .shell_wnd
      .borrow_mut()
      .set_ime_cursor_area(rect);
    self
  }

  pub fn set_ime_allowed(&self, allowed: bool) -> &Self {
    self
      .shell_wnd
      .borrow_mut()
      .set_ime_allowed(allowed);
    self
  }

  pub fn is_visible(&self) -> Option<bool> { self.shell_wnd.borrow().is_visible() }

  pub fn set_visible(&self, visible: bool) -> &Self {
    self.shell_wnd.borrow_mut().set_visible(visible);
    self
  }

  pub fn request_resize(&self, size: Size) { self.shell_wnd.borrow_mut().request_resize(size) }

  pub fn size(&self) -> Size { self.shell_wnd.borrow().inner_size() }

  pub fn set_min_size(&self, size: Size) -> &Self {
    self.shell_wnd.borrow_mut().set_min_size(size);
    self
  }

  pub fn is_pre_editing(&self) -> bool { self.pre_edit.borrow().is_some() }

  pub fn force_exit_pre_edit(&self) {
    if self.is_pre_editing() {
      self.set_ime_allowed(false);
      self.processes_ime_pre_edit(ImePreEdit::End);
      if let Some(s) = self.pre_edit.borrow_mut().take() {
        self.processes_receive_chars(s);
      }
      self.set_ime_allowed(true);
    }
  }

  pub fn exit_pre_edit(&self) {
    if self.is_pre_editing() {
      self.processes_ime_pre_edit(ImePreEdit::End);
      self.pre_edit.borrow_mut().take();
    }
  }

  pub fn update_pre_edit(&self, txt: &str, cursor: &Option<(usize, usize)>) {
    if !self.is_pre_editing() {
      self.processes_ime_pre_edit(ImePreEdit::Begin);
    }

    self.processes_ime_pre_edit(ImePreEdit::PreEdit { value: txt.to_owned(), cursor: *cursor });
    *self.pre_edit.borrow_mut() = Some(txt.to_owned());
  }

  fn once_on_lifecycle(
    &self, callback: impl FnOnce() + 'static, filter: impl Fn(&FrameMsg) -> bool + 'static,
  ) {
    let mut f = Some(callback);
    self
      .frame_ticker
      .clone()
      .filter(filter)
      .take(1)
      .subscribe(move |_| f.take().unwrap()());
  }
}

impl Drop for Window {
  // Safety: We retain ownership of the box in the constructor and release it
  // here.
  fn drop(&mut self) { let _release_tree = unsafe { Box::from_raw(self.tree.as_ptr()) }; }
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
  RemoveSubtree(WidgetId),
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
  KeyBoard {
    id: WidgetId,
    physical_key: PhysicalKey,
    key: VirtualKey,
    is_repeat: bool,
    location: KeyLocation,
    state: ElementState,
  },
  TabFocusMove,
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
  ImePreEdit {
    wid: WidgetId,
    pre_edit: ImePreEdit,
  },
  GrabPointerDown(WidgetId),
  GrabPointerMove(WidgetId),
  GrabPointerUp(WidgetId),
  BubbleCustomEvent {
    from: WidgetId,
    data: Box<dyn Any>,
  },
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

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn layout_after_wnd_resize() {
    reset_test_env!();

    let size = Size::new(100., 100.);
    let mut wnd = TestWindow::new_with_size(fn_widget! { MockBox { size: INFINITY_SIZE } }, size);
    wnd.draw_frame();
    wnd.assert_root_size(size);

    let new_size = Size::new(200., 200.);
    wnd.request_resize(new_size);

    wnd.draw_frame();
    wnd.assert_root_size(new_size);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn fire_tasks_before_new_window() {
    reset_test_env!();

    let theme = Theme::default();
    AppCtx::set_app_theme(theme);

    let mut wnd = TestWindow::new(fn_widget! {
      @MockBox {
        on_disposed: move |_| panic!("MockBox should not be disposed.\
          Since a theme is set before the window creation, \
          it should not trigger a regeneration."),
        size: Size::zero(),
      }
    });

    wnd.draw_frame();
  }
}
