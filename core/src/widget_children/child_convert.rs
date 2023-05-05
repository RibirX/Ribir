use super::{ComposeChild, ComposePair, DecorateTml, TemplateBuilder, TmlFlag, WidgetPair};
use crate::{
  dynamic_widget::{DynRender, DynWidget},
  state::{State, Stateful},
  widget::*,
};

/// Trait for conversions between child.
pub trait IntoChild<M: ImplMarker, Target> {
  fn into_child(self) -> Target;
}

/// Trait implement conversions for enum template.
pub trait IntoEnumVariable<M: ImplMarker, C> {
  fn into_variable(self) -> C;
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
impl<W> IntoChild<NotSelf<[(); 1]>, State<W>> for Stateful<W> {
  #[inline]
  fn into_child(self) -> State<W> { self.into() }
}

// Stateful<DynWidget<W>> -> State<W>
impl<W: 'static> IntoChild<NotSelf<[(); 1]>, State<W>> for Stateful<DynWidget<W>> {
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

// TemplateBuilder --> TemplateBuilder::Target
impl<T> IntoChild<NotSelf<[(); 5]>, T::Target> for T
where
  T: TemplateBuilder,
{
  #[inline]
  fn into_child(self) -> T::Target { self.build_tml() }
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

// WidgetPair<W, C> --> WidgetPair<W, C2>
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

// WidgetPair<W, C> --> WidgetPair<W2, C>
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

// WidgetPair<W, C> --> DecorateTml<W, C2>
impl<W, C, M, C2> IntoChild<NotSelf<[M; 8]>, DecorateTml<W, C2>> for WidgetPair<W, C>
where
  W: TmlFlag,
  C: IntoChild<M, C2>,
  M: ImplMarker,
{
  #[inline]
  fn into_child(self) -> DecorateTml<W, C2> {
    let WidgetPair { widget: tml_flag, child } = self;
    let decorator = Box::new(|w| w);
    DecorateTml {
      decorator,
      tml_flag,
      child: child.into_child(),
    }
  }
}

// ComposePair<W, C> --> DecorateTml<W, C2>
impl<W, C, T, C2, M1, M2> IntoChild<NotSelf<[(M1, M2); 8]>, DecorateTml<T, C2>>
  for ComposePair<State<W>, C>
where
  W: ComposeChild<Child = Widget> + 'static,
  W::Target: IntoWidget<M2>,
  C: IntoChild<M1, DecorateTml<T, C2>>,
  M1: ImplMarker,
  M2: ImplMarker,
  T: TmlFlag,
{
  #[inline]
  fn into_child(self) -> DecorateTml<T, C2> {
    let ComposePair { widget, child } = self;
    let DecorateTml { decorator, tml_flag, child } = child.into_child();
    DecorateTml {
      decorator: Box::new(move |w| ComposeChild::compose_child(widget, decorator(w)).into_widget()),
      tml_flag,
      child,
    }
  }
}

// ComposePair<W, C> --- W: ComposeChild---> ComposePair<W, W::Child>
impl<W, C> IntoChild<NotSelf<[(); 8]>, ComposePair<State<W>, W::Child>> for ComposePair<State<W>, C>
where
  W: ComposeChild,
  C: TemplateBuilder<Target = W::Child>,
{
  fn into_child(self) -> ComposePair<State<W>, W::Child> {
    let ComposePair { widget, child } = self;
    ComposePair { widget, child: child.build_tml() }
  }
}

// WidgetPair<W, C> --> DecorateTml<W, C2>
impl<W, C, W2, C2, M1, M2> IntoChild<NotSelf<[(M1, M2); 9]>, DecorateTml<W2, C2>>
  for WidgetPair<W, C>
where
  W: 'static,
  W2: TmlFlag,
  WidgetPair<W, Widget>: IntoWidget<M1>,
  C: IntoChild<M2, DecorateTml<W2, C2>>,
  M1: ImplMarker,
  M2: ImplMarker,
{
  #[inline]
  fn into_child(self) -> DecorateTml<W2, C2> {
    let Self { widget, child } = self;
    let DecorateTml { decorator, tml_flag, child } = child.into_child();
    DecorateTml {
      decorator: Box::new(move |w| WidgetPair { widget, child: decorator(w) }.into_widget()),
      tml_flag,
      child,
    }
  }
}

/////////////////////////////////////
pub(crate) trait FillVec<M, C> {
  fn fill_vec(self, vec: &mut Vec<C>);
}

// W --- C ---> Vec<C>
impl<W, C, M> FillVec<NotSelf<[M; 0]>, C> for W
where
  W: IntoChild<M, C>,
  M: ImplMarker,
{
  #[inline]
  fn fill_vec(self, vec: &mut Vec<C>) { vec.push(self.into_child()) }
}

// Iter<W> -- Iter<Option<C>> -> Vec<C>
impl<W, C, M> FillVec<NotSelf<[M; 1]>, C> for W
where
  W: IntoIterator,
  W::Item: IntoChild<M, Option<C>>,
  M: ImplMarker,
{
  #[inline]
  fn fill_vec(self, vec: &mut Vec<C>) {
    vec.extend(self.into_iter().filter_map(IntoChild::into_child))
  }
}

impl<D, M> FillVec<NotSelf<[M; 2]>, Widget> for Stateful<DynWidget<D>>
where
  D: IntoIterator + 'static,
  D::Item: IntoChild<M, Option<Widget>>,
  M: ImplMarker,
{
  fn fill_vec(self, vec: &mut Vec<Widget>) { vec.push(DynRender::new(self).into_widget()) }
}

/////////////////////////////////
// W -> W
impl<W> IntoEnumVariable<SelfImpl, W> for W {
  #[inline]
  fn into_variable(self) -> W { self }
}

// W -> State<W>
impl<W> IntoEnumVariable<NotSelf<[(); 1]>, State<W>> for W {
  #[inline]
  fn into_variable(self) -> State<W> { self.into() }
}

// Stateful<W> -> State<W>
impl<W> IntoEnumVariable<NotSelf<[(); 1]>, State<W>> for Stateful<W> {
  #[inline]
  fn into_variable(self) -> State<W> { self.into() }
}

// Stateful<DynWidget<W>> -> State<W>
impl<W: 'static> IntoEnumVariable<NotSelf<[(); 1]>, State<W>> for Stateful<DynWidget<W>> {
  #[inline]
  fn into_variable(self) -> State<W> { self.into() }
}

// WidgetPair<W, C> --> DecorateTml<W, C2>
impl<W, C, M, C2> IntoEnumVariable<NotSelf<[M; 2]>, DecorateTml<W, C2>> for WidgetPair<W, C>
where
  W: TmlFlag,
  C: IntoEnumVariable<M, C2>,
  M: ImplMarker,
{
  #[inline]
  fn into_variable(self) -> DecorateTml<W, C2> {
    let WidgetPair { widget: tml_flag, child } = self;
    let decorator = Box::new(|w| w);
    DecorateTml {
      decorator,
      tml_flag,
      child: child.into_variable(),
    }
  }
}

// WidgetPair<W, C> --> DecorateTml<W, C2>
impl<W, C, W2, C2, M1, M2> IntoEnumVariable<NotSelf<[(M1, M2); 2]>, DecorateTml<W2, C2>>
  for WidgetPair<W, C>
where
  W: 'static,
  W2: TmlFlag,
  WidgetPair<W, Widget>: IntoWidget<M1>,
  C: IntoEnumVariable<M2, DecorateTml<W2, C2>>,
  M1: ImplMarker,
  M2: ImplMarker,
{
  #[inline]
  fn into_variable(self) -> DecorateTml<W2, C2> {
    let Self { widget, child } = self;
    let DecorateTml { decorator, tml_flag, child } = child.into_variable();
    DecorateTml {
      decorator: Box::new(move |w| WidgetPair { widget, child: decorator(w) }.into_widget()),
      tml_flag,
      child,
    }
  }
}

impl<W, C, T, C2, M1, M2> IntoEnumVariable<NotSelf<[(M1, M2); 2]>, DecorateTml<T, C2>>
  for ComposePair<State<W>, C>
where
  W: ComposeChild<Child = Widget> + 'static,
  W::Target: IntoWidget<M2>,
  C: IntoEnumVariable<M1, DecorateTml<T, C2>>,
  M1: ImplMarker,
  M2: ImplMarker,
  T: TmlFlag,
{
  #[inline]
  fn into_variable(self) -> DecorateTml<T, C2> {
    let ComposePair { widget, child } = self;
    let DecorateTml { decorator, tml_flag, child } = child.into_variable();
    DecorateTml {
      decorator: Box::new(move |w| ComposeChild::compose_child(widget, decorator(w)).into_widget()),
      tml_flag,
      child,
    }
  }
}

impl<W, C> IntoEnumVariable<NotSelf<[(); 8]>, ComposePair<State<W>, W::Child>>
  for ComposePair<State<W>, C>
where
  W: ComposeChild,
  C: TemplateBuilder<Target = W::Child>,
{
  fn into_variable(self) -> ComposePair<State<W>, W::Child> {
    let ComposePair { widget, child } = self;
    ComposePair { widget, child: child.build_tml() }
  }
}

// WidgetPair<W, C> --> WidgetPair<W2, C2>
impl<M1, M2, W, W2, C, C2> IntoEnumVariable<NotSelf<[(M1, M2); 2]>, WidgetPair<W2, C2>>
  for WidgetPair<W, C>
where
  C: IntoEnumVariable<NotSelf<M1>, C2>,
  W: IntoEnumVariable<NotSelf<M2>, W2>,
{
  #[inline]
  fn into_variable(self) -> WidgetPair<W2, C2> {
    let Self { widget, child } = self;
    WidgetPair {
      widget: widget.into_variable(),
      child: child.into_variable(),
    }
  }
}

// WidgetPair<W, C> --> WidgetPair<W, C2>
impl<M, W, C, C2> IntoEnumVariable<NotSelf<[M; 3]>, WidgetPair<W, C2>> for WidgetPair<W, C>
where
  C: IntoEnumVariable<NotSelf<M>, C2>,
{
  #[inline]
  fn into_variable(self) -> WidgetPair<W, C2> {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_variable() }
  }
}

// WidgetPair<W, C> --> WidgetPair<W2, C>
impl<M, W, W2, C> IntoEnumVariable<NotSelf<[M; 4]>, WidgetPair<W2, C>> for WidgetPair<W, C>
where
  W: IntoEnumVariable<NotSelf<M>, W2>,
{
  #[inline]
  fn into_variable(self) -> WidgetPair<W2, C> {
    let Self { widget, child } = self;
    WidgetPair {
      widget: widget.into_variable(),
      child,
    }
  }
}
