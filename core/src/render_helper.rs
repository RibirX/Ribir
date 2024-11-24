use std::cell::RefCell;

use ribir_algo::Sc;
use smallvec::SmallVec;
use state_cell::StateCell;

use crate::prelude::*;

pub trait RenderProxy {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized>;
}
pub(crate) struct PureRender<R: Render>(pub R);

impl<R: Render> Query for PureRender<R> {
  fn query_all<'q>(&'q self, _: &QueryId, _: &mut SmallVec<[QueryHandle<'q>; 1]>) {}

  fn query_all_write<'q>(&'q self, _: &QueryId, _: &mut SmallVec<[QueryHandle<'q>; 1]>) {}

  fn query(&self, _: &QueryId) -> Option<QueryHandle> { None }

  fn query_write(&self, _: &QueryId) -> Option<QueryHandle> { None }

  fn query_match(
    &self, _: &[QueryId], _: &dyn Fn(&QueryId, &QueryHandle) -> bool,
  ) -> Option<(QueryId, QueryHandle)> {
    None
  }

  fn queryable(&self) -> bool { false }
}

impl<R: Render> RenderProxy for PureRender<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { &self.0 }
}

impl<T> Render for T
where
  T: RenderProxy + 'static,
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
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.borrow() }
}

impl<R: Render> RenderProxy for StateCell<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.read() }
}

impl<R: Render> RenderProxy for Sc<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self }
}

impl Render for Resource<Path> {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let line_width = ctx.painting_style().line_width();
    let size = self
      .bounds(line_width)
      .max()
      .to_vector()
      .to_size();
    clamp.clamp(size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let path = PaintPath::Share(self.clone());
    ctx.painter().draw_path(path);
  }
}
