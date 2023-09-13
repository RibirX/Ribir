use super::{ComposeChild, ComposePair, SinglePair};
use crate::{
  builtin_widgets::{FatObj, Void},
  prelude::Pipe,
  state::{State, StateFrom},
  widget::*,
};

/// Trait for conversions type as a compose child.
pub trait ChildFrom<V, M> {
  fn child_from(value: V) -> Self;
}

impl<T> ChildFrom<T, ()> for T {
  #[inline]
  fn child_from(value: T) -> Self { value }
}

impl<C, T: FromAnother<C, M>, M> ChildFrom<C, (M,)> for T {
  #[inline]
  fn child_from(value: C) -> Self { FromAnother::from_another(value) }
}

/// Convert from another type to the compose child.
pub trait FromAnother<V, M> {
  fn from_another(value: V) -> Self;
}

// W -> Widget
impl<W: StrictBuilder + 'static> FromAnother<W, ()> for Widget {
  #[inline]
  fn from_another(value: W) -> Self { value.into() }
}

impl<W: Into<Widget> + 'static> FromAnother<Pipe<Option<W>>, ()> for Widget {
  fn from_another(value: Pipe<Option<W>>) -> Self {
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
  fn from_another(value: T) -> Self { Self::state_from(value) }
}

// W --- C ---> Option<C>
impl<W, C, M> ChildFrom<W, [M; 1]> for Option<C>
where
  C: ChildFrom<W, M>,
{
  #[inline]
  fn child_from(value: W) -> Self { Some(ChildFrom::child_from(value)) }
}

// Option<W>  ---> Option<C>
impl<W, C, M> FromAnother<Option<C>, M> for Option<W>
where
  W: FromAnother<C, M>,
{
  #[inline]

  fn from_another(value: Option<C>) -> Self { value.map(FromAnother::from_another) }
}

// WidgetPair<W, C> --> WidgetPair<W2, C2>
impl<W, W2, C, C2, M1, M2> FromAnother<SinglePair<W2, C2>, [(M1, M2); 0]> for SinglePair<W, C>
where
  W: FromAnother<W2, M1>,
  C: FromAnother<C2, M2>,
{
  #[inline]
  fn from_another(value: SinglePair<W2, C2>) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair {
      widget: W::from_another(widget),
      child: C::from_another(child),
    }
  }
}

impl<W, W2, C, M> FromAnother<SinglePair<W2, C>, [M; 1]> for SinglePair<W, C>
where
  W: FromAnother<W2, M>,
{
  #[inline]
  fn from_another(value: SinglePair<W2, C>) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair { widget: W::child_from(widget), child }
  }
}

impl<W, C, C2, M> FromAnother<SinglePair<W, C2>, [M; 2]> for SinglePair<W, C>
where
  C: FromAnother<C2, M>,
{
  #[inline]
  fn from_another(value: SinglePair<W, C2>) -> Self {
    let SinglePair { widget, child } = value;
    SinglePair {
      widget,
      child: C::from_another(child),
    }
  }
}

// ComposePair<W, C> --- W: ComposeChild---> ComposePair<W, W::Child>
impl<W, C, C2, M> FromAnother<ComposePair<State<W>, C2>, M> for ComposePair<State<W>, C>
where
  W: ComposeChild,
  C: FromAnother<C2, M>,
{
  fn from_another(value: ComposePair<State<W>, C2>) -> Self {
    let ComposePair { widget, child } = value;
    ComposePair {
      widget,
      child: FromAnother::from_another(child),
    }
  }
}

impl<T1, T2, M> FromAnother<FatObj<T1>, [M; 0]> for FatObj<T2>
where
  T2: FromAnother<T1, M>,
{
  fn from_another(value: FatObj<T1>) -> Self {
    let (host, builtin) = value.unzip();
    FatObj::new(T2::from_another(host), builtin)
  }
}

impl<T1, T2, M> FromAnother<T1, [M; 1]> for FatObj<T2>
where
  T2: ChildFrom<T1, M>,
{
  #[inline]
  fn from_another(value: T1) -> Self { FatObj::from_host(ChildFrom::child_from(value)) }
}

pub(crate) trait FillVec<C, M> {
  fn fill_vec(self, vec: &mut Vec<C>);
}

impl<W, C: ChildFrom<W, M>, M> FillVec<C, [M; 0]> for W {
  #[inline]
  fn fill_vec(self, vec: &mut Vec<C>) { vec.push(ChildFrom::child_from(self)) }
}

impl<W, C, M> FillVec<C, [M; 1]> for W
where
  W: IntoIterator,
  C: ChildFrom<W::Item, M>,
{
  #[inline]
  fn fill_vec(self, vec: &mut Vec<C>) { vec.extend(self.into_iter().map(ChildFrom::child_from)) }
}
