use std::cell::RefCell;

use ribir_algo::Sc;
use state_cell::StateCell;

use crate::prelude::*;

pub(crate) trait RenderTarget {
  type Target: Render + ?Sized;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V;
}

pub(crate) struct RenderProxy<R: RenderTarget>(R);

impl<R: RenderTarget + Query> RenderProxy<R> {
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

impl<R: RenderTarget + Query> Render for RenderProxy<R> {
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

impl<R: RenderTarget + Query> Query for RenderProxy<R> {
  crate::widget::impl_proxy_query!(0);
}

impl<R: Render> RenderTarget for RefCell<R> {
  type Target = R;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.borrow()) }
}

impl<R: Render> RenderTarget for StateCell<R> {
  type Target = R;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.read()) }
}

impl<R: RenderTarget> RenderTarget for Sc<R> {
  type Target = R::Target;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { self.deref().proxy(f) }
}
