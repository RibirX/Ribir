use std::cell::RefCell;

use crate::prelude::*;

/// A container that enables efficient reuse of widget instances across multiple
/// placements.
///
/// Prefer using [`ReuseId`] for widget reuse between scopes instead of manually
/// managing reuse.
///
/// This type maintains a widget that can be recycled when removed from the
/// widget tree, allowing subsequent reuse through `get_widget()`. The widget is
/// fully disposed when:
///
/// - The `Reusable` instance is dropped
/// - `release()` is explicitly called
/// - The associated window is closed
///
/// Unlike `GenWidget` which creates new instances on each use, `Reusable`
/// maintains a single instance that gets rebuilt only on first use.
///
/// Note: if you get a widget from the `Reusable` instance, you must place it in
/// the widget tree. Otherwise, if maybe introducing a memory leak if it a
/// recycled widget.
///
/// # Panics
/// - When attempting to reuse across different windows
/// - Concurrent usage in multiple locations
/// - Calling `get_widget()` in Dropped state
/// - Window disposal before widget release
#[derive(Clone)]
pub struct Reusable(Rc<RefCell<ReusableInner>>);

impl Reusable {
  /// Creates a new Reusable container from a widget.
  ///
  /// returns`(Widget, Reusable)`, the Widget must be placed before the Reusable
  /// being used again, otherwise it will panic.
  pub fn new<'a, K>(w: impl IntoWidget<'a, K>) -> (Widget<'a>, Self) {
    let mut obj = FatObj::new(w);
    let track_id = obj.track_id();
    let this =
      Self(Rc::new(RefCell::new(ReusableInner { track_id, phase: ReusablePhase::Pending })));

    (this.attach_reusable_host(obj.into_widget()), this)
  }

  pub fn is_in_used(&self) -> bool {
    let inner_state = self.0.borrow();
    matches!(inner_state.phase, ReusablePhase::Active { .. })
  }

  /// Retrieves a widget instance for placement in the UI.
  ///
  /// Returns either:
  /// - A new instance on first use
  /// - A recycled instance if available
  ///
  /// # Panics
  /// - If the widget is already active in the UI
  /// - If called after `release()` or during window disposal
  /// - When crossing window boundaries
  pub fn get_widget(&self) -> Widget<'static> {
    let mut this = self.clone();
    fn_widget! {
      this.gen_widget()
    }
    .into_widget()
  }

  // The widget is generated lazily when `get_widget` is called, with actual
  // instantiation deferred until needed via `gen_widget`.
  fn gen_widget(&mut self) -> Widget<'static> {
    let mut inner_state = self.0.borrow_mut();
    let track_id = inner_state.track_id.clone();
    let w = match &mut inner_state.phase {
      ReusablePhase::Pending => {
        panic!("Reusable must be used before it can be retrieved")
      }
      ReusablePhase::Active { host } => {
        let host = host.clone();
        let track_id = track_id.clone();
        Widget::from_fn(move |ctx| {
          host
            .clone()
            .rehost(track_id.get().unwrap(), ctx.tree_mut())
        })
      }
      ReusablePhase::Cached { host, preserve } => {
        let host = host.clone();
        let preserve = preserve
          .take()
          .expect("recycled widget already taken for reuse");
        inner_state.phase = ReusablePhase::Active { host };
        preserve.into_widget()
      }
      ReusablePhase::Dropped => {
        panic!("Widget in invalid state for reuse. Expected Init/Recycled")
      }
    };

    self.attach_reusable_host(w)
  }

  /// Permanently disposes of the managed widget and associated resources.
  ///
  /// Subsequent calls to `get_widget()` will panic. Prefer dropping the
  /// `Reusable` instance unless manual cleanup timing is critical.
  pub fn release(&mut self) {
    let mut inner_state = self.0.borrow_mut();
    inner_state.phase = ReusablePhase::Dropped;
  }

  /// Attach a stable host wrapper around the reusable instance.
  ///
  /// The host implementation lives in the preserve lifecycle layer, but
  /// `Reusable` still decides when to attach that host and when to recycle the
  /// preserved subtree.
  fn attach_reusable_host<'a>(&self, widget: Widget<'a>) -> Widget<'a> {
    let mut fat = FatObj::new(widget);
    let this = self.clone();
    fat.on_disposed(move |e| {
      if matches!(this.0.borrow().phase, ReusablePhase::Active { .. }) {
        let (render, track_id) = match &*this.0.borrow() {
          ReusableInner { track_id, phase: ReusablePhase::Active { host }, .. } => {
            (host.clone(), track_id.clone())
          }
          _ => panic!("Widget in invalid state for reuse. Expected Active"),
        };
        if track_id.get() != Some(e.current_target()) {
          return;
        }
        let preserve = e.preserve();
        this.0.borrow_mut().phase =
          ReusablePhase::Cached { host: render, preserve: Some(preserve) };
      }
    });
    let this = self.clone();
    fat
      .into_widget()
      .on_build(move |id| this.on_build(id))
  }

  fn on_build(&self, id: WidgetId) {
    let host = PreserveHost::install(id, BuildCtx::get_mut().tree_mut());

    let mut state = self.0.borrow_mut();
    state.phase = ReusablePhase::Active { host };
  }
}

/// Internal state of a [`Reusable`] container.
struct ReusableInner {
  track_id: TrackId,
  phase: ReusablePhase,
}

/// The lifecycle phase of a reusable widget.
enum ReusablePhase {
  /// Initial state before the widget is used.
  Pending,
  /// Widget is currently mounted in the tree.
  Active { host: PreserveHost },
  /// Widget is detached and preserved for future use.
  Cached { host: PreserveHost, preserve: Option<Preserve> },
  /// Widget has been permanently released.
  Dropped,
}

impl Drop for Reusable {
  fn drop(&mut self) {
    if self.0.strong_count() == 1 {
      self.release();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::{TestWindow, split_value};

  #[test]
  fn test_reusable() {
    reset_test_env!();

    let (info, w_info) = split_value(vec![]);
    let (w_trigger, trigger) = split_value(0);
    let ctrl = Rc::new(RefCell::new(None));
    let ctrl2 = ctrl.clone();
    let wnd = TestWindow::from_widget(fn_widget! {
      let (w, reusable) = Reusable::new(@Text {
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
}
