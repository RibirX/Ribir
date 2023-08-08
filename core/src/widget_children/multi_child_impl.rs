use super::*;
use crate::widget::WidgetBuilder;

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

pub struct MultiPair<P> {
  pub parent: P,
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

impl<D> FillVec for Pipe<Multi<D>>
where
  D: IntoIterator + 'static,
  D::Item: Into<Widget>,
{
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    self.build_multi(vec, ctx.force_as_mut());
  }
}

impl<P, C> MultiWithChild<C> for P
where
  P: MultiChild,
  C: FillVec,
{
  type Target = MultiPair<P>;

  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let mut children = vec![];
    child.fill_vec(&mut children, ctx);
    MultiPair { parent: self, children }
  }
}

impl<C, P> MultiWithChild<C> for MultiPair<P>
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

impl<P: MultiParent> WidgetBuilder for MultiPair<P> {
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let MultiPair { parent, children } = self;
    parent.append_children(children, ctx.force_as_mut())
  }
}
