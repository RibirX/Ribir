use std::{cell::UnsafeCell, ptr::NonNull};

use smallvec::SmallVec;

use super::*;
use crate::{
  impl_common_event_deref,
  prelude::*,
  render_helper::{PureRender, RenderProxy},
  widget::widget_id::RenderQueryable,
  window::WindowId,
};

/// The event fired when the widget is mounted or performed layout.
pub type LifecycleEvent = CommonEvent;

/// The event fired when a widget enters the disposal pipeline.
///
/// Unlike [`LifecycleEvent`], this event can be intercepted by calling
/// [`DisposedEvent::preserve`] to detach the current subtree and keep it alive.
pub struct DisposedEvent {
  common: CommonEvent,
  preserved: bool,
}

impl_common_event_deref!(DisposedEvent);

impl DisposedEvent {
  pub(crate) fn new(target: WidgetId, tree: NonNull<WidgetTree>) -> Self {
    Self { common: CommonEvent::new(target, tree), preserved: false }
  }

  /// Prevents the widget and its subtree from being dropped and returns a
  /// [`Preserve`] handle to manage its lifetime.
  ///
  /// This is useful for animations (like fade-out) where a widget needs to stay
  /// in the tree even after its logical removal.
  pub fn preserve(&mut self) -> Preserve {
    assert!(!self.preserved, "preserve() called twice in one disposed event");

    self.common.prevent_default();
    self.preserved = true;

    let id = self.current_target();
    let wnd = self.window();
    let tree = wnd.tree_mut();
    assert_ne!(id, tree.root(), "preserve() cannot be used on the live window root");
    tree.prepare_subtree_for_dispose(id, false);

    Preserve::new(id, wnd.id())
  }

  /// Returns `true` if this event's target has been preserved via
  /// [`preserve`](Self::preserve).
  pub fn is_preserved(&self) -> bool { self.preserved }
}

/// A handle that keeps a detached widget subtree alive.
///
/// The subtree will be disposed of when this handle is dropped, unless it is
/// converted back into a [`Widget`] and reinserted into a window's widget tree.
pub struct Preserve {
  id: Option<WidgetId>,
  wnd_id: WindowId,
}

impl Preserve {
  fn new(id: WidgetId, wnd_id: WindowId) -> Self { Self { id: Some(id), wnd_id } }
}

impl From<Preserve> for Widget<'static> {
  fn from(mut value: Preserve) -> Self {
    let wnd_id = value.wnd_id;
    Widget::from_fn(move |ctx| {
      assert_eq!(
        wnd_id,
        ctx.window().id(),
        "Preserve widget must be used in the same window it was preserved in."
      );
      value
        .id
        .take()
        .expect("Preserve handle already consumed")
    })
  }
}

impl Drop for Preserve {
  fn drop(&mut self) {
    if let Some(id) = self.id.take()
      && let Some(wnd) = AppCtx::get_window(self.wnd_id)
    {
      let tree = wnd.tree();
      // If the widget is already dropped or has a parent (reinserted), we don't
      // need to dispose it.
      if !id.is_dropped(tree) && id.tree_parent(tree).is_none() {
        id.dispose_subtree(wnd.tree_mut());
      }
    }
  }
}

/// Stable host helper shared by preserve-based reuse implementations.
///
/// `Preserve` owns subtree lifetime, while `PreserveHost` keeps the outer host
/// shape stable across detach/reinsert cycles so repeated wrapping does not
/// accumulate render layers.
#[derive(Clone)]
pub(crate) struct PreserveHost(Rc<UnsafeCell<Box<dyn RenderQueryable>>>);

impl PreserveHost {
  pub(crate) fn install(id: WidgetId, tree: &mut WidgetTree) -> Self {
    let host = Self::default();
    id.wrap_node(tree, |render_node| {
      host.set(render_node);
      Box::new(host.clone())
    });
    host
  }

  /// Rehosts the inner render node into a new widget ID and moves all children
  /// from the `origin` widget to this new widget.
  pub(crate) fn rehost(self, origin: WidgetId, tree: &mut WidgetTree) -> WidgetId {
    let new_id = tree.alloc_node(self.take());
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

  fn as_render_node(&self) -> &dyn RenderQueryable {
    // SAFETY: PreserveHost is only used in the main thread and the UnsafeCell
    // provides a stable address for the Boxed render node.
    unsafe { &*self.0.get() }.as_ref()
  }

  fn set(&self, new_node: Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable> {
    // SAFETY: PreserveHost is only used in the main thread.
    unsafe { std::mem::replace(&mut *self.0.get(), new_node) }
  }

  fn take(&self) -> Box<dyn RenderQueryable> {
    // SAFETY: PreserveHost is only used in the main thread.
    unsafe { std::mem::replace(&mut *self.0.get(), Box::new(PureRender(Void::default()))) }
  }
}

impl RenderProxy for PreserveHost {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.as_render_node() }
}

impl Query for PreserveHost {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.as_render_node().query_all(query_id, out);
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self
      .as_render_node()
      .query_all_write(query_id, out);
  }

  fn query<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.as_render_node().query(query_id)
  }

  fn query_write<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.as_render_node().query_write(query_id)
  }

  fn queryable(&self) -> bool { true }
}

impl Default for PreserveHost {
  fn default() -> Self { Self(Rc::new(UnsafeCell::new(Box::new(PureRender(Void::default()))))) }
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn full_lifecycle() {
    reset_test_env!();

    let trigger = Stateful::new(0);
    let lifecycle = Stateful::new(vec![]);
    let c_lc = lifecycle.clone_reader();
    let c_trigger = trigger.clone_writer();
    let (is_empty, clean_trigger) = split_value(false);

    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        @ {
          pipe!(*$read(is_empty)).map(move |v| {
            (!v).then(move || fn_widget!{
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| $write(lifecycle).push("static mounted"),
                on_performed_layout: move |_| $write(lifecycle).push("static performed layout"),
                on_disposed: move |_| $write(lifecycle).push("static disposed"),
                @ {
                  pipe!(*$read(trigger)).map(move |_| fn_widget!{
                    @MockBox {
                      size: Size::zero(),
                      on_mounted: move |_| $write(lifecycle).push("dyn mounted"),
                      on_performed_layout: move |_| $write(lifecycle).push("dyn performed layout"),
                      on_disposed: move |_| $write(lifecycle).push("dyn disposed")
                    }
                  })
                }
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    assert_eq!(&**c_lc.read(), ["static mounted", "dyn mounted",]);

    wnd.draw_frame();

    assert_eq!(
      &**c_lc.read(),
      ["static mounted", "dyn mounted", "dyn performed layout", "static performed layout",]
    );
    {
      *c_trigger.write() += 1;
    }
    wnd.draw_frame();
    assert_eq!(
      &**c_lc.read(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
      ]
    );

    {
      *clean_trigger.write() = true;
    }
    wnd.draw_frame();
    assert_eq!(
      &**c_lc.read(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "static disposed",
        "dyn disposed"
      ]
    );
  }

  #[test]
  fn track_lifecycle() {
    reset_test_env!();

    let cnt = Stateful::new(3);
    let mounted: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());
    let disposed: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());

    let c_cnt = cnt.clone_writer();
    let c_mounted = mounted.clone_reader();
    let c_disposed = disposed.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(cnt)).map(move |cnt| {
            (0..cnt).map(move |_| {
              @MockBox {
                size: Size::zero(),
                on_mounted: move |e| { $write(mounted).insert(e.id); },
                on_disposed: move |e| { $write(disposed).insert(e.id); },
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();
    let mounted_ids = c_mounted.read().clone();

    *c_cnt.write() = 5;
    wnd.draw_frame();

    assert_eq!(mounted_ids.len(), 3);
    assert_eq!(&mounted_ids, &*c_disposed.read());
  }

  #[test]
  fn disposed_can_map_to_global_with_detached_parent() {
    reset_test_env!();

    let show = Stateful::new(true);
    let mounted_id = Stateful::new(None::<WidgetId>);
    let disposed_pos = Stateful::new(None::<Point>);

    let c_show = show.clone_writer();
    let c_mounted_id = mounted_id.clone_reader();
    let c_disposed_pos = disposed_pos.clone_reader();

    let w = fn_widget! {
      @MockBox {
        size: Size::new(200., 200.),
        x: 37.,
        y: 23.,
        @ {
          pipe!(*$read(show)).map(move |visible| {
            if visible {
              @MockBox {
                size: Size::new(50., 50.),
                x: 11.,
                y: 13.,
                on_mounted: move |e| *$write(mounted_id) = Some(e.id),
                on_disposed: move |e| *$write(disposed_pos) = Some(e.map_to_global(Point::zero())),
              }.into_widget()
            } else {
              @Void {}.into_widget()
            }
          })
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(400., 400.));
    wnd.draw_frame();
    let id = c_mounted_id
      .read()
      .expect("child should be mounted before dispose");
    let expected = wnd.map_to_global(Point::zero(), id);

    *c_show.write() = false;
    wnd.draw_frame();

    assert_eq!(*c_disposed_pos.read(), Some(expected));
  }

  #[test]
  fn preserve_root_skip_remove_until_drop() {
    reset_test_env!();

    let keep = Stateful::new(None::<Preserve>);
    let lifecycle = Stateful::new(vec![]);
    let mounted_id = Stateful::new(None::<WidgetId>);
    let keep_reader = keep.clone_reader();
    let keep_writer = keep.clone_writer();
    let lifecycle_reader = lifecycle.clone_reader();
    let mounted_id_reader = mounted_id.clone_reader();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::zero(),
          on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          on_event: move |e| {
            if matches!(e, Event::Disposed(_)) {
              $write(lifecycle).push(format!("dispose {:?}", e.current_target()));
            }
          },
          on_disposed: move |e| {
            let preserve = e.preserve();
            *$write(keep) = Some(preserve);
          },
        }
      },
      Size::new(100., 100.),
    );

    wnd.draw_frame();
    let id = mounted_id_reader
      .read()
      .expect("preserved root should mount");
    id.dispose_subtree(wnd.tree_mut());
    wnd.draw_frame();

    assert!(keep_reader.read().is_some());
    assert!(!id.is_dropped(wnd.tree()));
    assert_eq!(lifecycle_reader.read().len(), 1);

    keep_writer.write().take();
    wnd.draw_frame();

    assert!(id.is_dropped(wnd.tree()));
    assert!(!lifecycle_reader.read().is_empty());
  }

  #[test]
  fn preserve_descendant_survives_outer_remove() {
    reset_test_env!();

    let show = Stateful::new(true);
    let keep = Stateful::new(None::<Preserve>);
    let kept_id = Stateful::new(None::<WidgetId>);
    let keep_reader = keep.clone_reader();
    let show_writer = show.clone_writer();
    let kept_id_reader = kept_id.clone_reader();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::zero(),
          @ {
            pipe!(*$read(show)).map(move |show| {
              show.then(|| {
                @MockBox {
                  size: Size::zero(),
                  @MockBox {
                    size: Size::zero(),
                    on_mounted: move |e| *$write(kept_id) = Some(e.current_target()),
                    on_disposed: move |e| *$write(keep) = Some(e.preserve()),
                  }
                }
              })
            })
          }
        }
      },
      Size::new(100., 100.),
    );

    wnd.draw_frame();
    let id = kept_id_reader
      .read()
      .expect("kept child should mount");
    *show_writer.write() = false;
    wnd.draw_frame();

    assert!(keep_reader.read().is_some());
    assert!(!id.is_dropped(wnd.tree()));
  }
}
