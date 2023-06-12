use super::*;
use crate::widget::{RenderFul, WidgetBuilder};

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

pub struct MultiChildWidget {
  pub widget: Box<dyn Render>,
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

trait IntoMultiParent {
  fn into_multi_parent(self) -> Box<dyn Render>;
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

impl<R: MultiChild + Render + 'static> IntoMultiParent for R {
  #[inline]
  fn into_multi_parent(self) -> Box<dyn Render> { Box::new(self) }
}

impl<R: MultiChild + Render + 'static> IntoMultiParent for Stateful<R> {
  #[inline]
  fn into_multi_parent(self) -> Box<dyn Render> { Box::new(RenderFul(self)) }
}

impl<R, C> MultiWithChild<C> for R
where
  R: IntoMultiParent,
  C: FillVec,
{
  type Target = MultiChildWidget;

  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let mut children = vec![];
    child.fill_vec(&mut children, ctx);
    MultiChildWidget {
      widget: self.into_multi_parent(),
      children,
    }
  }
}

impl<C> MultiWithChild<C> for MultiChildWidget
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

impl WidgetBuilder for MultiChildWidget {
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let MultiChildWidget { widget, children } = self;
    let p = ctx.alloc_widget(widget);
    children
      .into_iter()
      .for_each(|child| ctx.append_child(p, child));
    p
  }
}
