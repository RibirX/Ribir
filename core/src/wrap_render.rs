use ribir_geom::{Point, Size, Transform};
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::prelude::*;

/// This trait is for a render widget that does not need to be an independent
/// node in the widget tree. It can serve as a wrapper for another render
/// widget.
///
/// # Which widgets should implement this trait?
///
/// If a render widget accepts a single child and its layout size matches its
/// child size, it can be implemented as a `WrapRender` instead of `Render`,
/// eliminating the need to allocate a node in the widget tree.
pub trait WrapRender {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) { host.paint(ctx) }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    // Detected by its host by default, so we return true here.
    host.size_affected_by_child()
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    host.hit_test(ctx, pos)
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> { host.get_transform() }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    host.visual_box(ctx)
  }

  fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase { host.dirty_phase() }

  fn combine_child(
    this: impl StateWriter<Value = Self>, mut child: Widget, dirty: DirtyPhase,
  ) -> Widget
  where
    Self: Sized + 'static,
  {
    let wrapper: Box<dyn WrapRender> = match this.try_into_value() {
      Ok(this) => Box::new(this),
      Err(this) => {
        let reader = match this.into_reader() {
          Ok(r) => r,
          Err(s) => {
            child = child.dirty_on(s.raw_modifies(), dirty);
            s.clone_reader()
          }
        };
        Box::new(reader)
      }
    };

    child.on_build(|id| {
      id.wrap_node(BuildCtx::get_mut().tree_mut(), move |r| {
        Box::new(RenderPair { wrapper, host: r })
      });
    })
  }
}

struct RenderPair {
  wrapper: Box<dyn WrapRender>,
  host: Box<dyn RenderQueryable>,
}

impl Query for RenderPair {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.host.query_all(query_id, out)
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.host.query_all_write(query_id, out)
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> { self.host.query(query_id) }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.host.query_write(query_id)
  }

  fn queryable(&self) -> bool { self.host.queryable() }
}

impl Render for RenderPair {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self
      .wrapper
      .perform_layout(clamp, self.host.as_render(), ctx)
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    self
      .wrapper
      .visual_box(self.host.as_render(), ctx)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { self.wrapper.paint(self.host.as_render(), ctx); }

  fn size_affected_by_child(&self) -> bool {
    self
      .wrapper
      .size_affected_by_child(self.host.as_render())
  }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self
      .wrapper
      .hit_test(self.host.as_render(), ctx, pos)
  }

  fn dirty_phase(&self) -> DirtyPhase { self.wrapper.dirty_phase(self.host.as_render()) }

  fn get_transform(&self) -> Option<Transform> { self.wrapper.get_transform(self.host.as_render()) }
}

impl<R> WrapRender for R
where
  R: StateReader,
  R::Value: WrapRender,
{
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    self.read().perform_layout(clamp, host, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) { self.read().paint(host, ctx) }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    self.read().size_affected_by_child(host)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self.read().hit_test(host, ctx, pos)
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    self.read().get_transform(host)
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    self.read().visual_box(host, ctx)
  }
}

#[macro_export]
macro_rules! impl_compose_child_for_wrap_render {
  ($name:ty, $dirty:expr) => {
    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;
      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        WrapRender::combine_child(this, child, $dirty)
      }
    }
  };
}

pub(crate) use impl_compose_child_for_wrap_render;
