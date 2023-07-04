use super::*;
use crate::widget::WidgetBuilder;

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

pub struct MultiPair {
  pub parent: WidgetId,
  pub children: Vec<WidgetId>,
}

/// A struct hold multi object in it, as a representative of multiple children,
/// so the parent know combined children one by one in it.
pub struct Multi<W>(W);

impl<W: IntoIterator> Multi<W> {
  #[inline]
  pub fn new(v: W) -> Self { Self(v) }

  #[inline]
  pub fn into_inner(self) -> W { self.0 }
}

// Same with ChildConvert::FillVec, but only for MultiChild,
// There are some duplicate implementations, but better compile error for
// users
trait FillVec {
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx);
}

impl<W: Into<Widget>> FillVec for W {
  #[inline]
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) { vec.push(self.into().build(ctx)) }
}

impl<W: Into<Widget>> FillVec for Option<W> {
  #[inline]
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    if let Some(w) = self {
      vec.push(w.into().build(ctx))
    }
  }
}

impl<W> FillVec for Multi<W>
where
  W: IntoIterator,
  W::Item: Into<Widget>,
{
  #[inline]
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    vec.extend(self.0.into_iter().map(|w| w.into().build(ctx)))
  }
}

impl<D> FillVec for Stateful<DynWidget<Multi<D>>>
where
  D: IntoIterator + 'static,
  Widget: From<D::Item>,
{
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    vec.push(DynRender::multi(self).build(ctx))
  }
}

impl<D> FillVec for Pipe<Multi<D>>
where
  D: IntoIterator + 'static,
  D::Item: Into<Widget>,
{
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    self.build_multi(vec, ctx.force_as_mut());
  }
}

trait MultiParent {
  fn build_as_parent(self, ctx: &mut BuildCtx) -> WidgetId;
}

impl<R: Into<Box<dyn Render>> + MultiChild> MultiParent for R {
  #[inline]
  fn build_as_parent(self, ctx: &mut BuildCtx) -> WidgetId { ctx.alloc_widget(self.into()) }
}

impl<W: Into<Box<dyn Render>> + MultiChild> MultiParent for Pipe<W> {
  #[inline]
  fn build_as_parent(self, ctx: &mut BuildCtx) -> WidgetId { self.build_as_render_parent(ctx) }
}

impl<R, C> MultiWithChild<C> for R
where
  R: MultiParent,
  C: FillVec,
{
  type Target = MultiPair;

  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let parent = self.build_as_parent(ctx.force_as_mut());
    let mut children = vec![];
    child.fill_vec(&mut children, ctx);
    MultiPair { parent, children }
  }
}

impl<C> MultiWithChild<C> for MultiPair
where
  C: FillVec,
{
  type Target = Self;
  #[inline]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    child.fill_vec(&mut self.children, ctx);
    self
  }
}

impl WidgetBuilder for MultiPair {
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let MultiPair { parent, children } = self;
    children
      .into_iter()
      .for_each(|child| ctx.force_as_mut().append_child(parent, child));

    parent
  }
}
