use super::{ComposeChild, ComposePair, SinglePair};
use crate::{
  builtin_widgets::{FatObj, Void},
  context::BuildCtx,
  prelude::Pipe,
  state::{State, StateFrom},
  widget::*,
};

/// Trait for conversions type as a compose child.
pub trait ChildFrom<V, M> {
  fn child_from(value: V, ctx: &BuildCtx) -> Self;
}

impl<T> ChildFrom<T, ()> for T {
  #[inline]
  fn child_from(value: T, _: &BuildCtx) -> Self { value }
}

impl<C, T: FromAnother<C, M>, M> ChildFrom<C, (M,)> for T {
  #[inline]
  fn child_from(value: C, ctx: &BuildCtx) -> Self { FromAnother::from_another(value, ctx) }
}

/// Convert from another type to the compose child.
pub trait FromAnother<V, M> {
  fn from_another(value: V, _: &BuildCtx) -> Self;
}

// W -> Widget
impl<W: StrictBuilder + 'static> FromAnother<W, ()> for Widget {
  #[inline]
  fn from_another(value: W, _: &BuildCtx) -> Self { value.into() }
}

impl<W: Into<Widget> + 'static> FromAnother<Pipe<Option<W>>, ()> for Widget {
  fn from_another(value: Pipe<Option<W>>, _: &BuildCtx) -> Self {
    value
      .map(|w| w.map_or_else(|| Widget::from(Void), |w| w.into()))
      .into()
  }
}

// W -> State<W>
// Stateful<W> -> State<W>
// Stateful<DynWidget<W>> -> State<W>
impl<W, T> FromAnother<T, ()> for State<W>
where
  State<W>: StateFrom<T>,
{
  #[inline]
  fn from_another(value: T, _: &BuildCtx) -> Self { Self::state_from(value) }
}

// W --- C ---> Option<C>
impl<W, C, M> FromAnother<W, [M; 0]> for Option<C>
where
  C: ChildFrom<W, M>,
{
  #[inline]
  fn from_another(value: W, ctx: &BuildCtx) -> Self { Some(ChildFrom::child_from(value, ctx)) }
}

// Option<W>  ---> Option<C>
impl<W, C, M> FromAnother<Option<C>, [M; 1]> for Option<W>
where
  W: FromAnother<C, M>,
{
  #[inline]
  fn from_another(value: Option<C>, ctx: &BuildCtx) -> Self {
    value.map(|v| FromAnother::from_another(v, ctx))
  }
}

// WidgetPair<W, C> --> WidgetPair<W2, C2>
impl<W, W2, C, C2, M1, M2> FromAnother<SinglePair<W2, C2>, [(M1, M2); 0]> for SinglePair<W, C>
where
  W: FromAnother<W2, M1>,
  C: FromAnother<C2, M2>,
{
  #[inline]
  fn from_another(value: SinglePair<W2, C2>, ctx: &BuildCtx) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair {
      widget: W::from_another(widget, ctx),
      child: C::from_another(child, ctx),
    }
  }
}

impl<W, W2, C, M> FromAnother<SinglePair<W2, C>, [M; 1]> for SinglePair<W, C>
where
  W: FromAnother<W2, M>,
{
  #[inline]
  fn from_another(value: SinglePair<W2, C>, ctx: &BuildCtx) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair {
      widget: W::child_from(widget, ctx),
      child,
    }
  }
}

impl<W, C, C2, M> FromAnother<SinglePair<W, C2>, [M; 2]> for SinglePair<W, C>
where
  C: FromAnother<C2, M>,
{
  #[inline]
  fn from_another(value: SinglePair<W, C2>, ctx: &BuildCtx) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair {
      widget,
      child: C::from_another(child, ctx),
    }
  }
}

// ComposePair<W, C> --- W: ComposeChild---> ComposePair<W, W::Child>
impl<W, C, C2, M> FromAnother<ComposePair<State<W>, C2>, M> for ComposePair<State<W>, C>
where
  W: ComposeChild,
  C: FromAnother<C2, M>,
{
  fn from_another(value: ComposePair<State<W>, C2>, ctx: &BuildCtx) -> Self {
    let ComposePair { widget, child } = value;
    ComposePair {
      widget,
      child: FromAnother::from_another(child, ctx),
    }
  }
}

impl<T1, T2, M> FromAnother<FatObj<T1>, [M; 0]> for FatObj<T2>
where
  T2: FromAnother<T1, M>,
{
  fn from_another(value: FatObj<T1>, ctx: &BuildCtx) -> Self {
    let (host, builtin) = value.unzip();
    FatObj::new(T2::from_another(host, ctx), builtin)
  }
}

impl<T1, T2, M> FromAnother<T1, [M; 1]> for FatObj<T2>
where
  T2: ChildFrom<T1, M>,
{
  #[inline]
  fn from_another(value: T1, ctx: &BuildCtx) -> Self {
    FatObj::from_host(ChildFrom::child_from(value, ctx))
  }
}
