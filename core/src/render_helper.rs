use crate::prelude::*;

pub(crate) trait RenderTarget {
  type Target: Render + ?Sized;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V;
}

pub(crate) struct RenderProxy<R: RenderTarget>(R);

impl<R: RenderTarget> RenderProxy<R> {
  #[inline]
  pub fn new(render: R) -> Self { Self(render) }
}

impl<R> SingleChild for RenderProxy<R>
where
  R: RenderTarget,
  R::Target: SingleChild,
{
}

impl<R> MultiChild for RenderProxy<R>
where
  R: RenderTarget,
  R::Target: MultiChild,
{
}

impl<R: RenderTarget + 'static> Render for RenderProxy<R> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.0.proxy(|r| r.perform_layout(clamp, ctx))
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.0.proxy(|r| r.paint(ctx)) }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.0.proxy(|r| r.only_sized_by_parent()) }

  #[inline]
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    self.0.proxy(|r| r.hit_test(ctx, pos))
  }

  #[inline]
  fn get_transform(&self) -> Option<Transform> { self.0.proxy(|r| r.get_transform()) }
}

impl<R: RenderTarget + 'static> Query for RenderProxy<R> {
  #[inline]
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.0.proxy(|r| r.query_inside_first(type_id, callback))
  }

  #[inline]
  fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.0.proxy(|r| r.query_outside_first(type_id, callback))
  }
}
