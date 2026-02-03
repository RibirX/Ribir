use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  ptr::NonNull,
};

use ribir_algo::Rc;
use smallvec::SmallVec;
use widget_id::TrackId;
use winit::event::{ElementState, Ime};
pub use winit::window::{CursorIcon, WindowLevel};

use crate::{
  events::{
    dispatcher::Dispatcher,
    focus_mgr::{FocusManager, FocusType},
  },
  prelude::*,
  ticker::{FrameMsg, FrameTicker},
};

/// The attributes use to create a window.
#[derive(Default)]
pub struct WindowAttributes(pub winit::window::WindowAttributes);

fn into_winit_size(size: Size) -> winit::dpi::Size {
  winit::dpi::LogicalSize::new(size.width, size.height).into()
}

impl WindowAttributes {
  /// Initial title of the window in the title bar.
  ///
  /// Default: `"Ribir App"`
  pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
    self.0.title = title.into();
    self
  }

  /// Whether the window should be resizable.
  ///
  /// Default: `true`
  pub fn with_resizable(&mut self, resizable: bool) -> &mut Self {
    self.0.resizable = resizable;
    self
  }

  /// Initial size of the window client area (excluding decorations).
  pub fn with_size(&mut self, size: Size) -> &mut Self {
    self.0.inner_size = Some(into_winit_size(size));
    self
  }

  /// Minimum size of the window client area
  pub fn with_min_size(&mut self, size: Size) -> &mut Self {
    self.0.min_inner_size = Some(into_winit_size(size));
    self
  }

  /// Maximum size of the window client area
  pub fn with_max_size(&mut self, size: Size) -> &mut Self {
    self.0.max_inner_size = Some(into_winit_size(size));
    self
  }

  /// Initial position of the window in screen coordinates.
  pub fn position(mut self, position: Point) -> Self {
    self.0.position = Some(winit::dpi::LogicalPosition::new(position.x, position.y).into());
    self
  }

  /// Whether the window should start maximized.
  ///
  /// Default: `false`
  pub fn with_maximized(&mut self, maximized: bool) -> &mut Self {
    self.0.maximized = maximized;
    self
  }

  /// Initial window visibility.
  ///
  /// Default: `true`
  pub fn with_visible(&mut self, visible: bool) -> &mut Self {
    self.0.visible = visible;
    self
  }

  /// Whether the window should show decorations.
  ///
  /// Default: `true`
  pub fn with_decorations(&mut self, decorations: bool) -> &mut Self {
    self.0.decorations = decorations;
    self
  }

  /// Window icon in RGBA8 format.
  pub fn with_icon(&mut self, icon: &PixelImage) -> &mut Self {
    debug_assert!(icon.color_format() == ColorFormat::Rgba8, "Icon must be in RGBA8 format");

    self.0.window_icon =
      winit::window::Icon::from_rgba(icon.pixel_bytes().to_vec(), icon.width(), icon.height()).ok();

    self
  }
}

pub enum UiEvent {
  RedrawRequest {
    wnd_id: WindowId,
    force: bool,
  },
  Resize {
    wnd_id: WindowId,
    size: Size,
  },
  ModifiersChanged {
    wnd_id: WindowId,
    state: ModifiersState,
  },
  CursorMoved {
    wnd_id: WindowId,
    pos: Point,
  },
  CursorLeft {
    wnd_id: WindowId,
  },
  MouseWheel {
    wnd_id: WindowId,
    delta_x: f32,
    delta_y: f32,
  },
  KeyBoard {
    wnd_id: WindowId,
    key: VirtualKey,
    state: ElementState,
    physical_key: PhysicalKey,
    is_repeat: bool,
    location: KeyLocation,
  },
  ImePreEdit {
    wnd_id: WindowId,
    ime: Ime,
  },
  ReceiveChars {
    wnd_id: WindowId,
    chars: CowArc<str>,
  },
  MouseInput {
    wnd_id: WindowId,
    device_id: Box<dyn DeviceId>,
    button: MouseButtons,
    state: ElementState,
  },
  CloseRequest {
    wnd_id: WindowId,
  },
}

impl UiEvent {
  pub fn wnd_id(&self) -> Option<WindowId> {
    match self {
      UiEvent::RedrawRequest { wnd_id, .. }
      | UiEvent::Resize { wnd_id, .. }
      | UiEvent::ModifiersChanged { wnd_id, .. }
      | UiEvent::CursorMoved { wnd_id, .. }
      | UiEvent::CursorLeft { wnd_id, .. }
      | UiEvent::MouseWheel { wnd_id, .. }
      | UiEvent::KeyBoard { wnd_id, .. }
      | UiEvent::ImePreEdit { wnd_id, .. }
      | UiEvent::ReceiveChars { wnd_id, .. }
      | UiEvent::MouseInput { wnd_id, .. }
      | UiEvent::CloseRequest { wnd_id } => Some(*wnd_id),
    }
  }
}

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
  pub(crate) running_animates: Rc<Cell<u32>>,
  pre_edit: RefCell<Option<String>>,
  /// This vector store the task to emit events. When perform layout, dispatch
  /// event and so on, some part of window may be already mutable borrowed and
  /// the user event callback may also query borrow that part, so we can't emit
  /// event immediately. So we store the event emitter in this vector,
  /// and emit them after all borrow finished.
  pub(crate) delay_emitter: RefCell<VecDeque<DelayEvent>>,

  /// A priority queue of tasks. So that tasks with lower priority value will be
  /// executed first.
  pub(crate) priority_task_queue: PriorityTaskQueue,
  shell_wnd: RefCell<BoxShellWindow>,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowId(u64);

pub trait Shell {
  fn new_shell_window(&self, attr: WindowAttributes) -> BoxFuture<'static, BoxShellWindow>;
  fn run_in_shell(&self, f: BoxFuture<'static, ()>);
  fn exit(&self);
}

#[cfg(target_arch = "wasm32")]
pub type BoxShell = Box<dyn Shell>;
#[cfg(not(target_arch = "wasm32"))]
pub type BoxShell = Box<dyn Shell + Send>;

pub trait ShellWindow {
  fn id(&self) -> WindowId;
  fn inner_size(&self) -> Size;
  fn set_ime_cursor_area(&mut self, rect: &Rect);
  fn set_ime_allowed(&mut self, allowed: bool);

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
  fn request_resize(&mut self, size: Size);
  fn set_window_level(&mut self, level: WindowLevel);
  // fn set_decorations(&mut self, decorations: bool);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn position(&self) -> Point;
  fn set_position(&mut self, point: Point);

  fn close(&self);

  /// Request a redraw. If `force` is true, the window will be redrawn even
  /// if nothing has changed.
  fn request_draw(&self, force: bool);

  fn draw_commands(
    &mut self, wnd_size: Size, viewport: Rect, surface_color: Color, commands: &[PaintCommand],
  );
}

#[cfg(target_arch = "wasm32")]
pub type BoxShellWindow = Box<dyn ShellWindow>;
#[cfg(not(target_arch = "wasm32"))]
pub type BoxShellWindow = Box<dyn ShellWindow + Send>;

impl Window {
  pub fn size(&self) -> Size { self.shell_wnd.borrow().inner_size() }

  pub fn process_keyboard_event(
    &self, physical_key: PhysicalKey, key: VirtualKey, is_repeat: bool, location: KeyLocation,
    state: ElementState,
  ) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_keyboard_input(physical_key, key, is_repeat, location, state);
  }

  pub fn process_receive_chars(&self, chars: CowArc<str>) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_receive_chars(chars)
  }

  pub fn process_wheel(&self, delta_x: f32, delta_y: f32) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_wheel(delta_x, delta_y);
  }

  pub fn process_cursor_move(&self, position: Point) {
    self
      .dispatcher
      .borrow_mut()
      .cursor_move_to(position);
  }

  pub fn process_cursor_leave(&self) { self.dispatcher.borrow_mut().on_cursor_leave(); }

  pub fn dispatch_ime_pre_edit(&self, ime: ImePreEdit) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_ime_pre_edit(ime)
  }

  pub fn process_mouse_press(&self, device_id: Box<dyn DeviceId>, button: MouseButtons) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_press_mouse(device_id, button);
  }

  pub fn process_mouse_release(&self, device_id: Box<dyn DeviceId>, button: MouseButtons) {
    self
      .dispatcher
      .borrow_mut()
      .dispatch_release_mouse(device_id, button);
  }

  /// Request switch the focus to next widget and return the actual focused
  /// widget ID on success.
  pub fn request_next_focus(&self, reason: FocusReason) -> Option<WidgetId> {
    self
      .focus_mgr
      .borrow_mut()
      .focus_next_widget(reason)
  }

  /// Attempts to set focus to the specified widget, returning the actual
  /// focused widget ID on success.
  pub fn request_focus(&self, wid: WidgetId, reason: FocusReason) -> Option<WidgetId> {
    self.focus_mgr.borrow_mut().focus(wid, reason)
  }

  /// Request switch the focus to previous widget and return the actual focused
  /// widget ID on success.
  pub fn request_prev_focus(&self, reason: FocusReason) -> Option<WidgetId> {
    self
      .focus_mgr
      .borrow_mut()
      .focus_prev_widget(reason)
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

  pub fn priority_task_queue(&self) -> &PriorityTaskQueue { &self.priority_task_queue }

  pub fn frame_tick_stream(&self) -> FrameTicker { self.frame_ticker.clone() }

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

  /// Draw an image what current render tree represent. **Note**: this function
  /// must be called after layout.
  #[track_caller]
  pub fn draw_frame(&self, wnd_size: Option<Size>) -> bool {
    let wnd_size = wnd_size.unwrap_or_else(|| self.size());
    let draw = !wnd_size.is_empty() && !self.tree().is_dirty();
    if draw {
      let root = self.tree().root();
      let surface = {
        let _guard = BuildCtx::init_for(root, self.tree);
        Palette::of(BuildCtx::get()).surface()
      };

      self.tree().draw();
      self.draw_delay_drop_widgets();

      let mut painter = self.painter.borrow_mut();

      #[cfg(feature = "debug")]
      {
        crate::debug_tool::paint_debug_overlays(self.id(), self.tree(), &mut painter);
      }
      let cmds = painter.finish();

      let mut shell = self.shell_wnd.borrow_mut();

      shell.draw_commands(wnd_size, Rect::from_size(wnd_size), surface, &cmds);
    }

    draw
  }

  pub fn layout(&self, size: Size) -> bool {
    let mut layout_queue = Vec::with_capacity(64);
    let mut notified_widgets = ahash::HashSet::default();
    let mut is_need_redraw = false;
    loop {
      self.run_frame_tasks();

      let tree = self.tree_mut();
      is_need_redraw |= tree.is_dirty();
      tree.layout(size, &mut layout_queue);

      // Process layout completion events
      layout_queue
        .drain(..)
        .filter(|id| notified_widgets.insert(*id))
        .for_each(|id| {
          self.add_delay_event(DelayEvent::PerformedLayout(id));
        });
      notified_widgets.clear();
      self.run_frame_tasks();

      if !tree.is_dirty() {
        self
          .focus_mgr
          .borrow_mut()
          .on_widget_tree_update(tree);
        self.run_frame_tasks();
      }

      if !tree.is_dirty() {
        let ready = FrameMsg::LayoutReady(Instant::now());
        self.frame_ticker.clone().next(ready);
        return is_need_redraw;
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

  pub fn new(shell_wnd: BoxShellWindow, flags: WindowFlags) -> Rc<Self> {
    let wnd_id = shell_wnd.id();
    let focus_mgr = RefCell::new(FocusManager::new(wnd_id));
    let tree = Box::new(WidgetTree::new(wnd_id));
    let dispatcher = RefCell::new(Dispatcher::new(wnd_id));

    let painter = Painter::new(Rect::from_size(shell_wnd.inner_size()));
    let window = Self {
      tree: NonNull::new(Box::into_raw(tree)).unwrap(),
      dispatcher,
      painter: RefCell::new(painter),
      focus_mgr,
      delay_emitter: <_>::default(),
      frame_ticker: Local::subject(),
      running_animates: <_>::default(),
      priority_task_queue: PriorityTaskQueue::default(),
      shell_wnd: RefCell::new(shell_wnd),
      delay_drop_widgets: <_>::default(),
      flags: Cell::new(flags),
      pre_edit: <_>::default(),
    };

    Rc::new(window)
  }

  pub fn position(&self) -> Point { self.shell_wnd.borrow().position() }

  pub fn set_position(&self, pos: Point) { self.shell_wnd.borrow_mut().set_position(pos); }

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

  pub fn shell_wnd(&self) -> &RefCell<BoxShellWindow> { &self.shell_wnd }

  pub fn flags(&self) -> WindowFlags { self.flags.get() }

  pub fn set_flags(&self, flags: WindowFlags) { self.flags.set(flags) }

  pub fn bubble_custom_event<E: 'static>(&self, from: WidgetId, e: E) {
    self.add_delay_event(DelayEvent::BubbleCustomEvent { from, data: Box::new(e) as Box<dyn Any> });
  }

  pub(crate) fn add_focus_node(
    this: Rc<Self>, track_id: TrackId, auto_focus: bool, focus_type: FocusType,
  ) -> FocusNodeGuard<impl Subscription> {
    let init_id = track_id.get().unwrap();
    let watcher = track_id.clone_watcher();

    this
      .focus_mgr
      .borrow_mut()
      .add_focus_node(init_id, auto_focus, focus_type);

    let wnd = this.clone();
    let guard = watch!(*$read(watcher))
      .merge(Local::of(Some(init_id)))
      .distinct_until_changed()
      .pairwise()
      .subscribe(move |(old, new)| {
        if let Some(wid) = old {
          wnd
            .focus_mgr
            .borrow_mut()
            .remove_focus_node(wid, focus_type);
        }
        if let Some(wid) = new {
          wnd
            .focus_mgr
            .borrow_mut()
            .add_focus_node(wid, auto_focus, focus_type);
        }
      })
      .unsubscribe_when_dropped();
    FocusNodeGuard { wnd: this, track_id, _guard: guard, focus_type }
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
          self.emit_from_outside(id, &mut e);
        }
        DelayEvent::PerformedLayout(id) => {
          let mut e = Event::PerformedLayout(LifecycleEvent::new(id, self.tree));
          self.emit_from_inside(id, &mut e);
        }
        DelayEvent::Disposed { id, parent } => {
          let mut stack = vec![id];
          while let Some(id) = stack.pop() {
            stack.extend(id.children(self.tree()));
            if Some(id) == self.focusing() {
              self
                .focus_mgr
                .borrow_mut()
                .blur(FocusReason::Other);
            }
            let mut e = Event::Disposed(LifecycleEvent::new(id, self.tree));
            self.emit_from_outside(id, &mut e);
          }

          let keep_alive_id = id
            .query_ref::<KeepAlive>(self.tree())
            .filter(|d| d.keep_alive)
            .map(|d| d.track_id());

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
        DelayEvent::Focus { id, reason } => {
          let mut e = Event::Focus(FocusEvent::new(id, reason, self.tree));
          self.emit_from_inside(id, &mut e);
        }
        DelayEvent::FocusIn { bottom, up, reason } => {
          let top = up.unwrap_or_else(|| self.tree().root());
          self.top_down_emit(
            &mut Event::FocusInCapture(FocusEvent::new(top, reason, self.tree)),
            bottom,
          );
          self.bottom_up_emit(&mut Event::FocusIn(FocusEvent::new(bottom, reason, self.tree)), up);
        }
        DelayEvent::Blur { id, reason } => {
          let mut e = Event::Blur(FocusEvent::new(id, reason, self.tree));
          self.emit_from_inside(id, &mut e);
        }
        DelayEvent::FocusOut { bottom, up, reason } => {
          let top = up.unwrap_or_else(|| self.tree().root());
          self.top_down_emit(
            &mut Event::FocusOutCapture(FocusEvent::new(top, reason, self.tree)),
            bottom,
          );
          self.bottom_up_emit(&mut Event::FocusOut(FocusEvent::new(bottom, reason, self.tree)), up);
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
          if let Event::KeyDown(e) = event
            && !e.is_prevent_default()
            && *e.key() == VirtualKey::Named(NamedKey::Tab)
          {
            self.add_delay_event(DelayEvent::TabFocusMove);
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
            focus_mgr.focus_prev_widget(FocusReason::Keyboard);
          } else {
            focus_mgr.focus_next_widget(FocusReason::Keyboard);
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
          self.emit_from_inside(wid, &mut e);
        }
        DelayEvent::GrabPointerMove(wid) => {
          let mut e = Event::PointerMove(PointerEvent::from_mouse(wid, self));
          self.emit_from_inside(wid, &mut e);
        }
        DelayEvent::GrabPointerUp(wid) => {
          let mut e = Event::PointerUp(PointerEvent::from_mouse(wid, self));
          self.emit_from_inside(wid, &mut e);
        }
        DelayEvent::BubbleCustomEvent { from: id, data } => {
          let mut e = Event::CustomEvent(new_custom_event(CommonEvent::new(id, self.tree), data));
          self.bottom_up_emit(&mut e, None);
        }
      }
    }
  }

  fn emit_from_inside(&self, id: WidgetId, e: &mut Event) {
    id.query_all_iter::<MixBuiltin>(self.tree())
      .any(|m| {
        if m.contain_flag(e.flags()) {
          m.dispatch(e);
        }
        e.is_prevent_default()
      });
  }

  fn emit_from_outside(&self, id: WidgetId, e: &mut Event) {
    id.query_all_iter::<MixBuiltin>(self.tree())
      .rev()
      .any(|m| {
        if m.contain_flag(e.flags()) {
          m.dispatch(e);
        }
        e.is_prevent_default()
      });
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
      self.emit_from_outside(*id, e);
      e.is_propagation()
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
        self.emit_from_inside(id, e);
        e.bubble_to_parent(id);
        e.is_propagation()
      });
  }

  /// Run all async tasks need finished in current frame and emit all delay
  /// events.
  pub fn run_frame_tasks(&self) {
    loop {
      if self.delay_emitter.borrow().is_empty() && self.priority_task_queue.is_empty() {
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

  /// Update the position of a widget. This is used by widgets like `Follow`
  /// that need to update position after layout is complete.
  pub(crate) fn update_widget_position(&self, id: WidgetId, pos: Point) {
    self
      .tree_mut()
      .store
      .layout_info_or_default(id)
      .pos = pos;
  }

  pub fn is_valid_widget(&self, id: WidgetId) -> bool { !id.is_dropped(self.tree()) }

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

  pub fn set_min_size(&self, size: Size) -> &Self {
    self.shell_wnd.borrow_mut().set_min_size(size);
    self
  }

  pub fn set_window_level(&self, level: WindowLevel) -> &Self {
    self
      .shell_wnd
      .borrow_mut()
      .set_window_level(level);
    self
  }

  pub fn is_pre_editing(&self) -> bool { self.pre_edit.borrow().is_some() }

  pub fn force_exit_pre_edit(&self) {
    if self.is_pre_editing() {
      self.set_ime_allowed(false);
      self.dispatch_ime_pre_edit(ImePreEdit::End);
      if let Some(s) = self.pre_edit.borrow_mut().take() {
        self.process_receive_chars(s.into());
      }
      self.set_ime_allowed(true);
    }
  }

  pub fn close(&self) {
    AppCtx::send_event(event_loop::FrameworkEvent::CloseWindow { wnd_id: self.id() });
  }

  pub(crate) fn dispose(&self) {
    self.tree_mut().disposed();
    self.run_frame_tasks();

    AppCtx::windows().borrow_mut().remove(&self.id());
    self.shell_wnd.borrow().close();
  }

  pub fn exit_pre_edit(&self) {
    if self.is_pre_editing() {
      self.dispatch_ime_pre_edit(ImePreEdit::End);
      self.pre_edit.borrow_mut().take();
    }
  }

  fn update_pre_edit(&self, txt: &str, cursor: &Option<(usize, usize)>) {
    if !self.is_pre_editing() {
      self.dispatch_ime_pre_edit(ImePreEdit::Begin);
    }

    self.dispatch_ime_pre_edit(ImePreEdit::PreEdit { value: txt.to_owned(), cursor: *cursor });
    *self.pre_edit.borrow_mut() = Some(txt.to_owned());
  }

  pub fn process_ime(&self, ime: Ime) {
    match ime {
      Ime::Enabled => {}
      Ime::Preedit(txt, cursor) => {
        if txt.is_empty() {
          self.exit_pre_edit();
        } else {
          self.update_pre_edit(&txt, &cursor);
        }
      }
      Ime::Commit(value) => {
        self.exit_pre_edit();
        self.process_receive_chars(value.into());
      }
      Ime::Disabled => self.exit_pre_edit(),
    }
  }

  fn once_on_lifecycle(
    &self, callback: impl FnOnce() + 'static, filter: impl Fn(&FrameMsg) -> bool + 'static,
  ) {
    let mut f = Some(callback);
    self
      .frame_ticker
      .clone()
      .filter(filter)
      .first()
      .subscribe(move |_| f.take().unwrap()());
  }
}

pub(crate) struct FocusNodeGuard<U: Subscription> {
  wnd: Rc<Window>,
  track_id: TrackId,
  focus_type: FocusType,
  _guard: SubscriptionGuard<U>,
}

impl<U: Subscription> Drop for FocusNodeGuard<U> {
  fn drop(&mut self) {
    if let Some(id) = self.track_id.get() {
      self
        .wnd
        .focus_mgr
        .borrow_mut()
        .remove_focus_node(id, self.focus_type);
    }
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
  Focus {
    id: WidgetId,
    reason: FocusReason,
  },
  Blur {
    id: WidgetId,
    reason: FocusReason,
  },
  FocusIn {
    bottom: WidgetId,
    up: Option<WidgetId>,
    reason: FocusReason,
  },
  FocusOut {
    bottom: WidgetId,
    up: Option<WidgetId>,
    reason: FocusReason,
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
    chars: CowArc<str>,
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
    let wnd = TestWindow::new_with_size(fn_widget! { MockBox { size: INFINITY_SIZE } }, size);
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

    AppCtx::set_app_theme(Theme::default());

    let (disposed, w_disposed) = split_value(false);
    TestWindow::from_widget(fn_widget! {
      @MockBox {
        on_disposed: move |_| *$write(w_disposed) = true,
        size: Size::zero(),
      }
    });

    AppCtx::run_until_stalled();
    assert!(
      !*disposed.read(),
      "MockBox should not be disposed.Since a theme is set before the window creation, it should \
       not trigger a regeneration."
    );
  }
}
