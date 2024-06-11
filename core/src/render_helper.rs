use std::cell::RefCell;

use ribir_algo::Sc;
use state_cell::{ReadRef, StateCell};

use crate::prelude::*;

pub trait RenderProxy {
  type R: ?Sized + Render;
  type Target<'r>: Deref<Target = Self::R>
  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_>;
}

pub(crate) struct PureRender<R: Render>(pub R);

impl<R: Render> Query for PureRender<R> {
  fn query_all(&self, _: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    smallvec::SmallVec::new()
  }

  fn query(&self, _: TypeId) -> Option<QueryHandle> { None }
}

impl<R: Render> RenderProxy for PureRender<R> {
  type R = R;
  type Target<'r> =&'r R where Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { &self.0 }
}

impl<T: RenderProxy + 'static> Render for T {
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
  type R = R;

  type Target<'r>  = std::cell::Ref<'r, R>
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.borrow() }
}

impl<R: Render> RenderProxy for StateCell<R> {
  type R = R;

  type Target<'r> = ReadRef<'r, R>
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.read() }
}

impl<R: Render> RenderProxy for Sc<R> {
  type R = R;

  type Target<'r> = &'r R
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self }
}

impl<R: Render> RenderProxy for Resource<R> {
  type R = R;

  type Target<'r> = &'r R
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self }
}
