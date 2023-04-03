use crate::{
  dynamic_widget::DynWidget,
  state::{State, Stateful},
  widget::*,
};

use super::WidgetPair;

/// Trait for conversions between child.
pub trait IntoChild<M: ImplMarker, Target> {
  fn into_child(self) -> Target;
}

// W -> W
impl<W> IntoChild<SelfImpl, W> for W {
  #[inline]
  fn into_child(self) -> W { self }
}

// W -> Widget
impl<W, M> IntoChild<NotSelf<[M; 0]>, Widget> for W
where
  W: IntoWidget<NotSelf<M>>,
{
  #[inline]
  fn into_child(self) -> Widget { self.into_widget() }
}

// W -> State<W>
impl<W> IntoChild<NotSelf<[(); 1]>, State<W>> for W {
  #[inline]
  fn into_child(self) -> State<W> { self.into() }
}

// Stateful<W> -> State<W>
impl<W> IntoChild<NotSelf<[(); 2]>, State<W>> for Stateful<W> {
  #[inline]
  fn into_child(self) -> State<W> { self.into() }
}

// Stateful<DynWidget<W>> -> State<W>
impl<W: 'static> IntoChild<NotSelf<[(); 2]>, State<W>> for Stateful<DynWidget<W>> {
  #[inline]
  fn into_child(self) -> State<W> { self.into() }
}

// Option<W> --- C(not W) ---> Option<C>
impl<W, C, M> IntoChild<NotSelf<[M; 3]>, Option<C>> for Option<W>
where
  W: IntoChild<NotSelf<M>, C>,
{
  #[inline]
  fn into_child(self) -> Option<C> { self.map(IntoChild::into_child) }
}

// W --- C ---> Option<C>
impl<W, C, M> IntoChild<NotSelf<[M; 4]>, Option<C>> for W
where
  W: IntoChild<M, C>,
  M: ImplMarker,
{
  #[inline]
  fn into_child(self) -> Option<C> { Some(self.into_child()) }
}

// W --- C ---> Vec<C>
impl<W, C, M> IntoChild<NotSelf<[M; 5]>, Vec<C>> for W
where
  W: IntoChild<M, C>,
  M: ImplMarker,
{
  #[inline]
  fn into_child(self) -> Vec<C> { vec![self.into_child()] }
}

// Iter<W> -- Iter<Option<C>> -> Vec<C>
impl<W, C, M> IntoChild<NotSelf<[M; 6]>, Vec<C>> for W
where
  W: IntoIterator,
  W::Item: IntoChild<M, Option<C>>,
  M: ImplMarker,
{
  #[inline]
  fn into_child(self) -> Vec<C> { self.into_iter().filter_map(|w| w.into_child()).collect() }
}

// WidgetPair<W, C> --> WidgetPair<W2, C2>
impl<M1, M2, W, W2, C, C2> IntoChild<NotSelf<[(M1, M2); 5]>, WidgetPair<W2, C2>>
  for WidgetPair<W, C>
where
  C: IntoChild<NotSelf<M1>, C2>,
  W: IntoChild<NotSelf<M2>, W2>,
{
  #[inline]
  fn into_child(self) -> WidgetPair<W2, C2> {
    let Self { widget, child } = self;
    WidgetPair {
      widget: widget.into_child(),
      child: child.into_child(),
    }
  }
}

impl<M, W, C, C2> IntoChild<NotSelf<[M; 6]>, WidgetPair<W, C2>> for WidgetPair<W, C>
where
  C: IntoChild<NotSelf<M>, C2>,
{
  #[inline]
  fn into_child(self) -> WidgetPair<W, C2> {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_child() }
  }
}

impl<M, W, W2, C> IntoChild<NotSelf<[M; 7]>, WidgetPair<W2, C>> for WidgetPair<W, C>
where
  W: IntoChild<NotSelf<M>, W2>,
{
  #[inline]
  fn into_child(self) -> WidgetPair<W2, C> {
    let Self { widget, child } = self;
    WidgetPair { widget: widget.into_child(), child }
  }
}
