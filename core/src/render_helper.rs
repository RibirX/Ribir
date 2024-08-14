use std::cell::RefCell;

use ribir_algo::Sc;
use smallvec::SmallVec;
use state_cell::StateCell;

use crate::prelude::*;

pub trait RenderProxy {
  type Target<'r>: Deref
  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_>;
}

pub(crate) struct PureRender<R: Render>(pub R);

impl<R: Render> Query for PureRender<R> {
  fn query_all<'q>(&'q self, _: TypeId, _: &mut SmallVec<[QueryHandle<'q>; 1]>) {}

  fn query(&self, _: TypeId) -> Option<QueryHandle> { None }

  fn query_write(&self, _: TypeId) -> Option<QueryHandle> { None }

  fn queryable(&self) -> bool { false }
}

impl<R: Render> RenderProxy for PureRender<R> {
  type Target<'r> =&'r R where Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { &self.0 }
}

impl<T> Render for T
where
  T: RenderProxy + 'static,
  for<'r> <T::Target<'r> as Deref>::Target: Render,
{
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.proxy().perform_layout(clamp, ctx)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.proxy().paint(ctx) }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.proxy().only_sized_by_parent() }

  #[inline]
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest { self.proxy().hit_test(ctx, pos) }

  #[inline]
  fn get_transform(&self) -> Option<Transform> { self.proxy().get_transform() }
}

impl<R: Render> RenderProxy for RefCell<R> {
  type Target<'r>  = std::cell::Ref<'r, R>
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.borrow() }
}

impl<R: Render> RenderProxy for StateCell<R> {
  type Target<'r> = ReadRef<'r, R>
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.read() }
}

impl<R: Render> RenderProxy for Sc<R> {
  type Target<'r> = &'r R
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self }
}

impl<R: Render> RenderProxy for Resource<R> {
  type Target<'r> = &'r R
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self }
}
