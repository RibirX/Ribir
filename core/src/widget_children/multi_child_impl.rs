use smallvec::SmallVec;

use super::*;
use crate::pipe::InnerPipe;

pub struct MultiPair {
  pub parent: Widget,
  pub children: SmallVec<[Widget; 1]>,
}

pub trait MultiIntoParent: IntoWidget<RENDER> {
  fn into_parent(self, ctx: &BuildCtx) -> Widget;
}

macro_rules! impl_widget_child {
  ($($m: ident),*) => {
    $(
      // Choose `IntoWidgetStrict` for child widgets instead of `IntoWidget`. This is
      // because `IntoWidget` may lead
      // `Pipe<Value = Option<impl IntoWidget>>` has two implementations:
      //
      // - As a single widget child, satisfy the `IntoWidget` requirement, albeit not
      //   `IntoWidget`.
      // - As a `Pipe` that facilitates iteration over multiple widgets.
      impl< C: IntoWidgetStrict<$m>> WithChild<C, 1, { 100 + $m }> for MultiPair
      {
        type Target = MultiPair;
        #[inline]
        fn with_child(mut self, child: C, ctx: &BuildCtx) -> MultiPair{
          self.children.push(child.into_widget_strict(ctx));
          self
        }
      }
    )*
  };
}

macro_rules! impl_iter_widget_child {
  ($($m: ident), *) => {
    $(
      impl<C> WithChild<C, 1, { 110 + $m }> for MultiPair
      where
        C:IntoIterator,
        C::Item: IntoWidget<$m>,
      {
        type Target = MultiPair;
        #[inline]
        fn with_child(mut self, child: C, ctx: &BuildCtx) -> MultiPair {
          self.children.extend(child.into_iter().map(|w| w.into_widget(ctx)));
          self
        }
      }
    )*
  };
}

macro_rules! impl_pipe_iter_widget_child {
  ($($m: ident), *) => {
    $(
      impl<C, V> WithChild<C, 1, { 120 + $m }> for MultiPair
      where
        C:InnerPipe<Value=V>,
        V:IntoIterator,
        V::Item: IntoWidget<$m>,
      {
        type Target = MultiPair;
        fn with_child(mut self, child: C, ctx: &BuildCtx) -> MultiPair {
          self.children.extend(child.build_multi(ctx));
          self
        }
      }
    )*
  };
}

impl_widget_child!(COMPOSE, RENDER, COMPOSE_CHILD, FN);
impl_iter_widget_child!(COMPOSE, RENDER, COMPOSE_CHILD, FN);
impl_pipe_iter_widget_child!(COMPOSE, RENDER, COMPOSE_CHILD, FN);

impl<C, T, const M: usize> WithChild<C, 1, M> for T
where
  T: MultiIntoParent,
  MultiPair: WithChild<C, 1, M>,
{
  type Target = <MultiPair as WithChild<C, 1, M>>::Target;
  #[inline]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    MultiPair { parent: self.into_parent(ctx), children: SmallVec::new() }.with_child(child, ctx)
  }
}

impl WithChild<Widget, 1, FN> for MultiPair {
  type Target = MultiPair;

  #[inline]
  fn with_child(mut self, child: Widget, _: &BuildCtx) -> MultiPair {
    self.children.push(child);
    self
  }
}

impl WidgetBuilder for MultiPair {
  fn build(self, ctx: &BuildCtx) -> Widget { self.into_widget_strict(ctx) }
}

impl IntoWidgetStrict<COMPOSE_CHILD> for MultiPair {
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    let MultiPair { parent, children } = self;
    let leaf = parent.id().single_leaf(&ctx.tree.borrow().arena);
    for c in children {
      ctx.append_child(leaf, c);
    }
    parent
  }
}

// Implementation `IntoParent`
impl<P: MultiChild + IntoWidget<RENDER>> MultiIntoParent for P {
  #[inline]
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_widget(ctx) }
}

impl<S, V, F> MultiIntoParent for MapPipe<V, S, F>
where
  S: Pipe,
  V: MultiIntoParent + 'static,
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<S, V, F> MultiIntoParent for FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
  V: MultiIntoParent + 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<V: MultiIntoParent + 'static> MultiIntoParent for Box<dyn Pipe<Value = V>> {
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<P: MultiIntoParent> MultiIntoParent for FatObj<P> {
  fn into_parent(self, ctx: &BuildCtx) -> Widget {
    self.map(|p| p.into_parent(ctx)).into_widget(ctx)
  }
}
