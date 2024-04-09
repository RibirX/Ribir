use super::*;
use crate::pipe::InnerPipe;

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<C, M: ?Sized> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

pub struct MultiPair<P> {
  pub parent: P,
  pub children: Vec<Widget>,
}

trait FillVec<M: ?Sized> {
  fn fill_vec(self, vec: &mut Vec<Widget>, ctx: &BuildCtx);
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<W: {#}> FillVec<dyn {#}> for W {
    #[inline]
    fn fill_vec(self, vec: &mut Vec<Widget>, ctx: &BuildCtx) { vec.push(self.build(ctx)) }
  }

  impl<W> FillVec<&dyn {#}> for W
  where
    W: IntoIterator,
    W::Item: {#},
  {
    #[inline]
    fn fill_vec(self, vec: &mut Vec<Widget>, ctx: &BuildCtx) {
      vec.extend(self.into_iter().map(|w| w.build(ctx)))
    }
  }
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<T, V> FillVec<&&dyn {#}> for T
  where
    T: InnerPipe<Value=V>,
    V: IntoIterator + 'static,
    V::Item: {#},
  {
    fn fill_vec(self, vec: &mut Vec<Widget>, ctx: &BuildCtx) {
      self.build_multi(vec, |v, ctx| v.build(ctx), ctx);
    }
  }
}

impl<M: ?Sized, P, C> MultiWithChild<C, M> for P
where
  P: MultiParent,
  C: FillVec<M>,
{
  type Target = MultiPair<P>;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let mut children = vec![];
    child.fill_vec(&mut children, ctx);
    MultiPair { parent: self, children }
  }
}

impl<M: ?Sized, C, P> MultiWithChild<C, M> for MultiPair<P>
where
  C: FillVec<M>,
{
  type Target = Self;
  #[inline]
  #[track_caller]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    child.fill_vec(&mut self.children, ctx);
    self
  }
}

impl<P: MultiParent> WidgetBuilder for MultiPair<P> {
  fn build(self, ctx: &BuildCtx) -> Widget {
    let MultiPair { parent, children } = self;
    parent.compose_children(children.into_iter(), ctx)
  }
}
