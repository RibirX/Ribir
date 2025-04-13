use std::cell::{RefCell, UnsafeCell};

use smallvec::SmallVec;

use crate::{
  prelude::*,
  render_helper::{PureRender, RenderProxy},
  widget::widget_id::RenderQueryable,
  window::WindowId,
};

/// A container that enables efficient reuse of widget instances across multiple
/// placements.
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
pub struct Reusable(Sc<RefCell<ReusableState>>);

impl Reusable {
  /// Creates a new Reusable container from a widget.
  ///
  /// Use `get_widget()` to retrieve a widget instance.
  pub fn new<const M: usize>(mut w: FatObj<impl IntoWidget<'static, M>>) -> Self {
    let track_id = w.get_track_id_widget().read().track_id();
    Self(Sc::new(RefCell::new(ReusableState::Init { track_id, widget: w.into_widget() })))
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
  pub fn get_widget(&mut self) -> FatObj<Widget<'static>> {
    let (w, track_id) = {
      let inner_state = &mut *self.0.borrow_mut();
      match inner_state {
        ReusableState::Init { track_id, widget: w } => {
          (std::mem::replace(w, Void.into_widget()), track_id.clone())
        }
        ReusableState::Recycled { track_id, wnd_id } => {
          let wnd_id = *wnd_id;
          let w = Widget::from_fn({
            let track_id = track_id.clone();
            move |ctx| {
              let current_wnd_id = ctx.window().id();
              assert_eq!(
                wnd_id, current_wnd_id,
                "Reusable widget must be used in the same window it was recycled in."
              );
              track_id.get().unwrap()
            }
          });
          (w, track_id.clone())
        }
        ReusableState::Building(_) | ReusableState::Dropped => {
          panic!("Widget in invalid state for reuse. Expected Init/Recycled")
        }
      }
    };
    self.handle_reusable_wrapper(track_id, w)
  }

  /// Permanently disposes of the managed widget and associated resources.
  ///
  /// Subsequent calls to `get_widget()` will panic. Prefer dropping the
  /// `Reusable` instance unless manual cleanup timing is critical.
  pub fn release(&mut self) {
    let mut inner_state = self.0.borrow_mut();
    if let ReusableState::Recycled { track_id, wnd_id } = &*inner_state {
      let wid = track_id.get().unwrap();
      let wnd = AppCtx::get_window(*wnd_id).unwrap_or_else(|| {
        panic!("Window for reusable widget already closed");
      });
      wid.dispose_subtree(wnd.tree_mut());
    }
    *inner_state = ReusableState::Dropped;
  }

  fn handle_reusable_wrapper(
    &self, track_id: TrackId, widget: Widget<'static>,
  ) -> FatObj<Widget<'static>> {
    let render_cell = ReusableRenderWrapper::default();
    let widget = widget.on_build({
      let render_cell = render_cell.clone();
      move |id| {
        id.wrap_node(BuildCtx::get_mut().tree_mut(), |render_node| {
          render_cell.set(render_node);
          Box::new(render_cell.clone())
        })
      }
    });

    let mut fat = FatObj::new(widget);
    let this = self.clone();
    fat.on_disposed(move |e| {
      // We will recycle the original widget, so we prevent to continue emitting
      // the `Disposed` event to the original widget.
      e.prevent_default();

      let wnd = e.window();
      let tree = wnd.tree_mut();
      let id = e.current_target();

      let original = render_cell.take();
      let reuse_id = tree.alloc_node(original);

      id.children(tree)
        .collect::<SmallVec<[WidgetId; 1]>>()
        .into_iter()
        .for_each(|child| reuse_id.append(child, tree));

      reuse_id
        .get_node_mut(tree)
        .unwrap()
        .update_track_id(reuse_id);

      let mut inner_state = this.0.borrow_mut();
      let ReusableState::Building(track_id) = &*inner_state else {
        panic!("Cannot recycle widget in a wrong state.");
      };

      *inner_state = ReusableState::Recycled { track_id: track_id.clone(), wnd_id: wnd.id() };
    });
    *self.0.borrow_mut() = ReusableState::Building(track_id);
    fat
  }
}

/// A wrapper around a render node that is used to split the original render
/// node and its outer wrapper, so we can take the original render node out to
/// be recycled and return the wrapper to be disposed.
#[derive(Clone)]
struct ReusableRenderWrapper(Sc<UnsafeCell<Box<dyn RenderQueryable>>>);

enum ReusableState {
  Init { track_id: TrackId, widget: Widget<'static> },
  Building(TrackId),
  Recycled { track_id: TrackId, wnd_id: WindowId },
  Dropped,
}

impl RenderProxy for ReusableRenderWrapper {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.as_render_node() }
}

impl Query for ReusableRenderWrapper {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.as_render_node().query_all(query_id, out);
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self
      .as_render_node()
      .query_all_write(query_id, out);
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.as_render_node().query(query_id)
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.as_render_node().query_write(query_id)
  }

  fn queryable(&self) -> bool { true }
}

impl ReusableRenderWrapper {
  fn as_render_node(&self) -> &dyn RenderQueryable { unsafe { &*self.0.get() }.as_ref() }

  fn set(&self, new_node: Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable> {
    unsafe { std::mem::replace(&mut *self.0.get(), new_node) }
  }

  fn take(&self) -> Box<dyn RenderQueryable> {
    unsafe { std::mem::replace(&mut *self.0.get(), Box::new(PureRender(Void))) }
  }
}

impl Default for ReusableRenderWrapper {
  fn default() -> Self { Self(Sc::new(UnsafeCell::new(Box::new(PureRender(Void))))) }
}

impl Drop for Reusable {
  fn drop(&mut self) {
    if self.0.ref_count() == 1 {
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
    let mut reusable = Reusable::new(FatObj::new(text! {
      text: "Hello",
      on_mounted: move |_| $w_info.write().push("Mounted"),
      on_disposed: move |_| $w_info.write().push("Disposed"),
    }));

    let (w_trigger, trigger) = split_value(true);
    let reusable2 = reusable.clone();
    let mut wnd = TestWindow::new(fn_widget! {
      let reusable = reusable2.clone();
      @ pipe!{
        let mut reusable = reusable.clone();
        fn_widget! {
          if *$w_trigger {
            reusable.get_widget().into_widget()
          } else {
            Void.into_widget()
          }
        }
      }
    });

    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() = false;
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() = true;
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted"]);

    *trigger.write() = false;
    wnd.draw_frame();
    reusable.release();
    wnd.draw_frame();
    assert_eq!(*info.read(), vec!["Mounted", "Disposed"]);
  }
}
