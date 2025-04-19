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
  /// returns`(Widget, Reusable)`, the Widget must be placed before the Reusable
  /// being used again, otherwise it will panic.
  pub fn new<'a, const M: usize>(w: impl IntoWidget<'a, M>) -> (Widget<'a>, Self) {
    let mut obj = FatObj::new(w);
    let track_id = obj.get_track_id_widget().read().track_id();
    let this = Self(Sc::new(RefCell::new(ReusableState::WaitToUse { track_id })));

    (this.handle_reusable_wrapper(obj.into_widget()), this)
  }

  pub fn is_in_used(&self) -> bool {
    let inner_state = self.0.borrow();
    matches!(*inner_state, ReusableState::Using { .. })
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
    let w = match &mut *inner_state {
      ReusableState::WaitToUse { .. } => {
        panic!("Reusable must be used before it can be retrieved")
      }
      ReusableState::Using { render, track_id } => {
        let render = render.clone();
        let track_id = track_id.clone();
        Widget::from_fn(move |ctx| {
          let reuse_id =
            move_inner_render_to_new(render.take(), track_id.get().unwrap(), ctx.tree_mut());
          reuse_id
        })
      }
      ReusableState::Recycled { track_id, wnd_id } => {
        let wnd_id = *wnd_id;
        Widget::from_fn({
          let track_id = track_id.clone();
          move |ctx| {
            let current_wnd_id = ctx.window().id();
            assert_eq!(
              wnd_id, current_wnd_id,
              "Reusable widget must be used in the same window it was recycled in."
            );
            track_id
              .get()
              .expect("reusable has no be used yet")
          }
        })
      }
      ReusableState::Dropped => {
        panic!("Widget in invalid state for reuse. Expected Init/Recycled")
      }
    };

    self.handle_reusable_wrapper(w)
  }

  /// Permanently disposes of the managed widget and associated resources.
  ///
  /// Subsequent calls to `get_widget()` will panic. Prefer dropping the
  /// `Reusable` instance unless manual cleanup timing is critical.
  pub fn release(&mut self) {
    let mut inner_state = self.0.borrow_mut();
    if let ReusableState::Recycled { track_id, wnd_id } = &*inner_state {
      let wid = track_id.get().unwrap();
      if let Some(wnd) = AppCtx::get_window(*wnd_id) {
        wid.dispose_subtree(wnd.tree_mut());
      }
    }
    *inner_state = ReusableState::Dropped;
  }

  fn handle_reusable_wrapper<'a>(&self, widget: Widget<'a>) -> Widget<'a> {
    let mut fat = FatObj::new(widget);
    let this = self.clone();
    fat.on_disposed(move |e| {
      if matches!(*this.0.borrow(), ReusableState::Using { .. }) {
        // We will recycle the original widget, so we prevent to continue emitting
        // the `Disposed` event to the original widget.
        let (render_cell, track_id) = match &*this.0.borrow() {
          ReusableState::Using { render, track_id, .. } => (render.clone(), track_id.clone()),
          _ => panic!("Widget in invalid state for reuse. Expected Using"),
        };
        if track_id.get() != Some(e.current_target()) {
          return;
        }

        e.prevent_default();

        let wnd = e.window();
        let tree = wnd.tree_mut();
        let id = e.current_target();

        move_inner_render_to_new(render_cell.take(), id, tree);

        *this.0.borrow_mut() =
          ReusableState::Recycled { track_id: track_id.clone(), wnd_id: wnd.id() };
      }
    });
    let this = self.clone();
    fat
      .into_widget()
      .on_build(move |id| this.on_build(id))
  }

  fn on_build(&self, id: WidgetId) {
    let track_id = match &*self.0.borrow() {
      ReusableState::Using { track_id, .. }
      | ReusableState::Recycled { track_id, .. }
      | ReusableState::WaitToUse { track_id } => track_id.clone(),
      _ => panic!("Widget in invalid state for reuse. Expected Init/Recycled"),
    };

    let render = ReusableRenderWrapper::default();
    id.wrap_node(BuildCtx::get_mut().tree_mut(), |render_node| {
      render.set(render_node);
      Box::new(render.clone())
    });

    *self.0.borrow_mut() = ReusableState::Using { track_id, render };
  }
}

fn move_inner_render_to_new(
  inner: Box<dyn RenderQueryable>, origin: WidgetId, tree: &mut WidgetTree,
) -> WidgetId {
  let new_id = tree.alloc_node(inner);
  origin
    .children(tree)
    .collect::<SmallVec<[WidgetId; 1]>>()
    .into_iter()
    .for_each(|child| new_id.append(child, tree));

  new_id
    .get_node_mut(tree)
    .unwrap()
    .update_track_id(new_id);

  new_id
}

/// A wrapper around a render node that is used to split the original render
/// node and its outer wrapper, so we can take the original render node out to
/// be recycled and return the wrapper to be disposed.
#[derive(Clone)]
struct ReusableRenderWrapper(Sc<UnsafeCell<Box<dyn RenderQueryable>>>);

enum ReusableState {
  WaitToUse { track_id: TrackId },
  Using { track_id: TrackId, render: ReusableRenderWrapper },
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
    let (w_trigger, trigger) = split_value(0);
    let ctrl = Sc::new(RefCell::new(None));
    let ctrl2 = ctrl.clone();
    let mut wnd = TestWindow::new(fn_widget! {
      let (w, reusable) = Reusable::new(@Text {
        text: "Hello",
        on_mounted: move |_| $w_info.write().push("Mounted"),
        on_disposed: move |_| $w_info.write().push("Disposed"),
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
          if *$w_trigger < 3 {
            f.gen_widget()
          } else {
            Void {}.into_widget()
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
