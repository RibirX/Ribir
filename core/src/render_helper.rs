use std::cell::RefCell;

use ribir_algo::Rc;
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

  fn query<'q>(&'q self, _: &QueryId) -> Option<QueryHandle<'q>> { None }

  fn query_write<'q>(&'q self, _: &QueryId) -> Option<QueryHandle<'q>> { None }

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
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    self.proxy().measure(clamp, ctx)
  }

  #[inline]
  fn place_children(&self, size: Size, ctx: &mut PlaceCtx) {
    self.proxy().place_children(size, ctx)
  }

  #[inline]
  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> { self.proxy().visual_box(ctx) }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.proxy().paint(ctx) }

  #[inline]
  fn size_affected_by_child(&self) -> bool { self.proxy().size_affected_by_child() }

  #[inline]
  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self.proxy().hit_test(ctx, pos)
  }

  #[inline]
  fn get_transform(&self) -> Option<Transform> { self.proxy().get_transform() }
}

impl<R: Render> RenderProxy for RefCell<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.borrow() }
}

impl<R: Render> RenderProxy for StateCell<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.read() }
}

impl<R: Render> RenderProxy for Rc<R> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self }
}

impl Render for Resource<Path> {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let line_width = Provider::of::<PaintingStyle>(ctx).and_then(|p| p.line_width());
    let size = self
      .bounds(line_width)
      .max()
      .to_vector()
      .to_size();
    clamp.clamp(size)
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { true }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let painter = ctx.painter();
    let path = PaintPath::Share(self.clone());
    if let Some(PaintingStyle::Stroke(options)) = style {
      painter.set_strokes(options).stroke_path(path);
    } else {
      painter.fill_path(path);
    }
  }
}
