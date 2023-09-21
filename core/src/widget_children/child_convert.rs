use super::Pair;
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
crate::widget::multi_build_replace_impl! {
  impl<W: {#} > FromAnother<W, &dyn {#}> for Widget {
    #[inline]
    fn from_another(value: W, ctx: &BuildCtx) -> Self { value.widget_build(ctx) }
  }
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<W: {#} + 'static> FromAnother<Pipe<Option<W>>, &dyn {#}> for Widget {
    fn from_another(value: Pipe<Option<W>>, ctx: &BuildCtx) -> Self {
      crate::pipe::pipe_option_to_widget!(value, ctx)
     }
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
impl<W, W2, C, C2, M1, M2> FromAnother<Pair<W2, C2>, [(M1, M2); 0]> for Pair<W, C>
where
  W: FromAnother<W2, M1>,
  C: FromAnother<C2, M2>,
{
  #[inline]
  fn from_another(value: Pair<W2, C2>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair {
      parent: W::from_another(widget, ctx),
      child: C::from_another(child, ctx),
    }
  }
}

impl<W, W2, C, M> FromAnother<Pair<W2, C>, [M; 1]> for Pair<W, C>
where
  W: FromAnother<W2, M>,
{
  #[inline]
  fn from_another(value: Pair<W2, C>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair {
      parent: W::child_from(widget, ctx),
      child,
    }
  }
}

impl<W, C, C2, M> FromAnother<Pair<W, C2>, [M; 2]> for Pair<W, C>
where
  C: FromAnother<C2, M>,
{
  #[inline]
  fn from_another(value: Pair<W, C2>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair {
      parent: widget,
      child: C::from_another(child, ctx),
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
