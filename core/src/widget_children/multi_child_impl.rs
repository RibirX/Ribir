use super::*;
use crate::widget::StrictBuilder;

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<C, M> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

pub struct MultiPair<P> {
  pub parent: P,
  pub children: Vec<WidgetId>,
}

// Same with ChildConvert::FillVec, but only for MultiChild,
// There are some duplicate implementations, but better compile error for
// users and `MultiChild` support `pipe<impl IntoIterator<Item = impl Widget>>`
// as child but `ComposeChild` not.
trait FillVec<M> {
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx);
}

impl<W: Into<Widget>> FillVec<()> for W {
  #[inline]
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) { vec.push(self.build(ctx)) }
}

impl<W> FillVec<[WidgetId; 0]> for W
where
  W: IntoIterator,
  W::Item: WidgetBuilder,
{
  #[inline]
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    vec.extend(self.into_iter().map(|w| w.build(ctx)))
  }
}

impl<D> FillVec<[(); 1]> for Pipe<D>
where
  D: IntoIterator + 'static,
  D::Item: WidgetBuilder,
{
  fn fill_vec(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx) {
    self.build_multi(vec, ctx.force_as_mut());
  }
}

impl<M, P, C> MultiWithChild<C, M> for P
where
  P: MultiChild,
  C: FillVec<M>,
{
  type Target = MultiPair<P>;

  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let mut children = vec![];
    child.fill_vec(&mut children, ctx);
    MultiPair { parent: self, children }
  }
}

impl<M, C, P> MultiWithChild<C, M> for MultiPair<P>
where
  C: FillVec<M>,
{
  type Target = Self;
  #[inline]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    child.fill_vec(&mut self.children, ctx);
    self
  }
}

impl<P: MultiParent> StrictBuilder for MultiPair<P> {
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
    let MultiPair { parent, children } = self;
    parent.append_children(children, ctx.force_as_mut())
  }
}
