use std::cell::RefCell;

use crate::prelude::*;

/// A high-level reusable widget for the common `'static` owned path.
///
/// `Reusable` owns the initial widget and lazily creates its underlying
/// [`ReuseHandle`] when the widget is first built. Subsequent builds reuse the
/// same preserved subtree through that handle.
#[derive(Clone)]
pub struct Reusable(std::rc::Rc<RefCell<ReusableInner>>);

enum ReusableInner {
  Initial(Widget<'static>),
  Reusing(ReuseHandle),
  Released,
}

/// A low-level handle that enables efficient reuse of widget instances across
/// multiple placements.
///
/// Prefer using [`ReuseId`] for widget reuse between scopes instead of manually
/// managing reuse.
///
/// This handle maintains a widget that can be recycled when removed from the
/// widget tree, allowing subsequent reuse through `get_widget()`. The widget is
/// fully disposed when:
///
/// - The `ReuseHandle` instance is dropped
/// - `release()` is explicitly called
/// - The associated window is closed
///
/// Unlike `GenWidget` which creates new instances on each use, `ReuseHandle`
/// maintains a single instance that gets rebuilt only on first use.
///
/// Note: if you get a widget from the `ReuseHandle` instance, you must place it
/// in the widget tree. Otherwise, it may introduce a memory leak if it is a
/// recycled widget.
///
/// # Panics
/// - When attempting to reuse across different windows
/// - Concurrent usage in multiple locations
/// - Calling `get_widget()` after `release()`
/// - Calling `get_widget()` before placing the initial widget
#[derive(Clone)]
pub struct ReuseHandle(std::rc::Rc<RefCell<ReuseHandleInner>>);

struct ReuseHandleInner {
  track_id: TrackId,
  state: ReuseHandleState,
}

enum ReuseHandleState {
  PendingMount,
  InUse { host: PreserveHost },
  Cached { host: PreserveHost, preserve: Preserve },
  Released,
}

impl ReuseHandleInner {
  fn is_in_use(&self) -> bool { matches!(self.state, ReuseHandleState::InUse { .. }) }

  fn should_capture_recycled(&self, target: WidgetId) -> bool {
    self.track_id.get() == Some(target) && matches!(self.state, ReuseHandleState::InUse { .. })
  }

  fn capture_recycled(&mut self, preserve: Preserve) {
    match &mut self.state {
      ReuseHandleState::InUse { host } => {
        self.state = ReuseHandleState::Cached { host: host.clone(), preserve };
      }
      ReuseHandleState::Cached { .. }
      | ReuseHandleState::PendingMount
      | ReuseHandleState::Released => {}
    }
  }

  fn prepare_widget(&mut self) -> (PreserveHost, Option<Preserve>) {
    match std::mem::replace(&mut self.state, ReuseHandleState::Released) {
      ReuseHandleState::PendingMount => {
        self.state = ReuseHandleState::PendingMount;
        panic!("Reusable widget not yet initialized. The initial widget must be placed first.")
      }
      ReuseHandleState::Released => panic!("Reusable widget has been released."),
      ReuseHandleState::InUse { host } => {
        self.state = ReuseHandleState::InUse { host: host.clone() };
        (host, None)
      }
      ReuseHandleState::Cached { host, preserve } => {
        self.state = ReuseHandleState::InUse { host: host.clone() };
        (host, Some(preserve))
      }
    }
  }

  fn bind_host(&mut self, host: PreserveHost) {
    if !matches!(self.state, ReuseHandleState::Released) {
      self.state = ReuseHandleState::InUse { host };
    }
  }

  fn release(&mut self) { self.state = ReuseHandleState::Released; }
}

impl ReuseHandle {
  /// Creates a new low-level reuse handle from a widget.
  ///
  /// Returns `(Widget, ReuseHandle)`. The returned Widget MUST be placed in the
  /// tree before the `ReuseHandle` can be used to retrieve it again.
  pub fn new<'a, K>(w: impl IntoWidget<'a, K>) -> (Widget<'a>, Self) {
    let mut obj = FatObj::new(w);
    let this = Self(std::rc::Rc::new(RefCell::new(ReuseHandleInner {
      track_id: obj.track_id(),
      state: ReuseHandleState::PendingMount,
    })));

    let state = this.0.clone();
    obj.on_event(move |event| {
      if let Event::Disposing(e) = event {
        let mut inner = state.borrow_mut();
        if inner.should_capture_recycled(e.current_target()) {
          inner.capture_recycled(e.preserve());
        }
      }
    });

    (this.bind_host(obj.into_widget()), this)
  }

  /// Returns true if the managed widget is currently active in the UI.
  pub fn is_in_use(&self) -> bool {
    let inner = self.0.borrow();
    inner.is_in_use()
  }

  /// Retrieves a widget instance for placement in the UI.
  ///
  /// Returns either:
  /// - A new instance on first use
  /// - A recycled instance if available
  ///
  /// # Panics
  /// - If called after `release()`
  /// - If called before the initial widget is placed
  pub fn get_widget(&self) -> Widget<'static> {
    let this = self.clone();
    fn_widget! { this.gen_widget() }.into_widget()
  }

  fn gen_widget(&self) -> Widget<'static> {
    let mut inner = self.0.borrow_mut();
    let track_id = inner.track_id.clone();
    let (host, recycled) = inner.prepare_widget();
    drop(inner);

    let widget = Widget::from_fn(move |ctx| {
      let id = host.rehost(track_id.get().unwrap(), ctx.tree_mut());
      drop(recycled);
      id
    });
    self.bind_host(widget)
  }

  /// Permanently disposes of the managed widget and associated resources.
  ///
  /// Subsequent calls to `get_widget()` will panic.
  pub fn release(&self) {
    let mut inner = self.0.borrow_mut();
    inner.release();
  }

  fn bind_host<'a>(&self, widget: Widget<'a>) -> Widget<'a> {
    let state = self.0.clone();
    widget.on_build(move |id| {
      let host = PreserveHost::install(id, BuildCtx::get_mut().tree_mut());
      state.borrow_mut().bind_host(host);
    })
  }
}

impl Reusable {
  /// Creates a reusable widget for the common `'static` owned path.
  pub fn new<K>(w: impl IntoWidget<'static, K>) -> Self {
    Self(std::rc::Rc::new(RefCell::new(ReusableInner::Initial(w.into_widget()))))
  }

  /// Returns true if the managed widget is currently active in the UI.
  pub fn is_in_use(&self) -> bool {
    match &*self.0.borrow() {
      ReusableInner::Initial(_) | ReusableInner::Released => false,
      ReusableInner::Reusing(handle) => handle.is_in_use(),
    }
  }

  /// Retrieves a widget instance for placement in the UI.
  pub fn get_widget(&self) -> Widget<'static> {
    let mut inner = self.0.borrow_mut();
    if let ReusableInner::Reusing(handle) = &*inner {
      return handle.get_widget();
    }

    match std::mem::replace(&mut *inner, ReusableInner::Released) {
      ReusableInner::Initial(widget) => {
        let (widget, handle) = ReuseHandle::new(widget);
        *inner = ReusableInner::Reusing(handle);
        widget
      }
      ReusableInner::Released => panic!("Reusable widget has been released."),
      ReusableInner::Reusing(_) => unreachable!(),
    }
  }

  /// Permanently disposes of the managed widget and associated resources.
  pub fn release(&self) {
    if let ReusableInner::Reusing(handle) =
      std::mem::replace(&mut *self.0.borrow_mut(), ReusableInner::Released)
    {
      handle.release();
    }
  }
}

impl From<Reusable> for Widget<'static> {
  fn from(value: Reusable) -> Self { value.get_widget() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    builtin_widgets::MixBuiltin,
    test_helper::{MockBox, TestWindow, split_value},
    window::UiEvent,
  };

  #[test]
  fn test_reusable() {
    reset_test_env!();

    let (info, w_info) = split_value(vec![]);
    let (w_trigger, trigger) = split_value(0);
    let ctrl = Rc::new(RefCell::new(None));
    let ctrl2 = ctrl.clone();
    let wnd = TestWindow::from_widget(fn_widget! {
      let (w, reusable) = ReuseHandle::new(@Text {
        text: "Hello",
        on_mounted: move |_| $write(w_info).push("Mounted"),
        on_disposed: move |_| $write(w_info).push("Disposed"),
      });
      *ctrl2.borrow_mut() = Some(reusable.clone());
      let mut w = Some(w.into_widget());
      let f = GenWidget::new(move || {
        if w.is_some() {
          w.take().unwrap()
        } else {
          reusable.get_widget()
        }
      });
      @ pipe!{
        let f = f.clone();
        fn_widget! {
          if *$read(w_trigger) < 3 {
            f.gen_widget()
          } else {
            Void::default().into_widget()
          }
        }
      }
    });

    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    ctrl.borrow_mut().as_mut().unwrap().release();
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted", "Disposed"]);
  }

  #[test]
  fn reusable_should_not_accumulate_recycle_mixins() {
    reset_test_env!();

    let (w_trigger, trigger) = split_value(0);
    let ctrl = Rc::new(RefCell::new(None));
    let ctrl2 = ctrl.clone();
    let wnd = TestWindow::from_widget(fn_widget! {
      let (w, reusable) = ReuseHandle::new(@Text { text: "Hello" });
      *ctrl2.borrow_mut() = Some(reusable.clone());
      let mut w = Some(w.into_widget());
      let f = GenWidget::new(move || {
        if w.is_some() {
          w.take().unwrap()
        } else {
          reusable.get_widget()
        }
      });
      @ pipe!{
        let f = f.clone();
        fn_widget! {
          if *$read(w_trigger) < 3 {
            f.gen_widget()
          } else {
            Void::default().into_widget()
          }
        }
      }
    });

    wnd.draw_frame();
    let reusable = ctrl.borrow().as_ref().unwrap().clone();
    let mix_count = mix_builtin_count(&reusable, &wnd);
    assert!(mix_count > 0);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert_eq!(mix_builtin_count(&reusable, &wnd), mix_count);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert_eq!(mix_builtin_count(&reusable, &wnd), mix_count);

    reusable.release();
    wnd.draw_frame();
  }

  fn mix_builtin_count(reusable: &ReuseHandle, wnd: &TestWindow) -> usize {
    let track_id = reusable.0.borrow().track_id.clone();
    track_id
      .get()
      .unwrap()
      .query_all_iter::<MixBuiltin>(wnd.tree())
      .count()
  }

  #[test]
  fn reusable_widget_lazy_build_and_reuse() {
    reset_test_env!();

    let (info, w_info) = split_value(vec![]);
    let (w_trigger, trigger) = split_value(0);
    let ctrl = Rc::new(RefCell::new(None));
    let ctrl2 = ctrl.clone();
    let pre_mount = Rc::new(RefCell::new(None));
    let pre_mount2 = pre_mount.clone();
    let wnd = TestWindow::from_widget(fn_widget! {
      let reusable = Reusable::new(@Text {
        text: "Hello",
        on_mounted: move |_| $write(w_info).push("Mounted"),
        on_disposed: move |_| $write(w_info).push("Disposed"),
      });
      *pre_mount2.borrow_mut() = Some(reusable.is_in_use());
      *ctrl2.borrow_mut() = Some(reusable.clone());
      let reusable = reusable.clone();
      let f = GenWidget::new(move || reusable.clone());
      @ pipe!{
        let f = f.clone();
        fn_widget! {
          if *$read(w_trigger) < 3 {
            f.gen_widget()
          } else {
            Void::default().into_widget()
          }
        }
      }
    });

    assert_eq!(*pre_mount.borrow(), Some(false));
    let reusable = ctrl.borrow().as_ref().unwrap().clone();

    wnd.draw_frame();
    assert!(reusable.is_in_use());
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert!(reusable.is_in_use());
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert!(reusable.is_in_use());
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() += 1;
    wnd.draw_frame();
    assert!(!reusable.is_in_use());
    assert_eq!(*info.read(), vec!["Mounted"]);

    reusable.release();
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted", "Disposed"]);
  }

  #[test]
  fn reusable_widget_release_before_first_mount_panics_on_use() {
    reset_test_env!();

    let reusable = Reusable::new(Text::new("Hello"));
    reusable.release();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      let reusable = reusable.clone();
      let wnd = TestWindow::from_widget(move || reusable.clone());
      wnd.draw_frame();
    }));

    assert!(result.is_err());
  }

  #[test]
  fn cached_reuse_handle_survives_window_close_cleanup() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! { @MockBox { size: Size::zero() } },
      Size::new(100., 100.),
    );
    let (tooltip, reusable) = ReuseHandle::new(Text::new("tip"));
    let tooltip = Rc::new(RefCell::new(Some(tooltip.into_widget())));

    let mount_tooltip = |x| {
      let tooltip = tooltip
        .borrow_mut()
        .take()
        .unwrap_or_else(|| reusable.get_widget());
      let mut tooltip = FatObj::new(tooltip);
      tooltip.with_x(x);
      wnd.mount(tooltip.into_widget())
    };

    let handle = mount_tooltip(10.);
    wnd.draw_frame();
    assert!(reusable.is_in_use());

    drop(handle);
    wnd.draw_frame();
    assert!(!reusable.is_in_use());

    assert!(AppCtx::send_ui_event(UiEvent::CloseRequest { wnd_id: wnd.id() }));
    AppCtx::run_until_stalled();
  }
}
