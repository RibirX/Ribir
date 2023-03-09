use crate::{
  dynamic_widget::DynWidget,
  state::{State, Stateful},
  widget::{ImplMarker, IntoWidget, NotSelf, Widget},
};

use super::{ComposeChild, WidgetPair};

/// Trait specify what child a compose child widget can have, and the target
/// type after widget compose its child.
pub trait ComposeWithChild<M, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

/// The pair a compose child widget with its child or not fill fulled child.
pub struct ComposePair<W, C> {
  pub widget: W,
  pub child: C,
}

// Template use to construct child of a widget.
pub trait Template: Sized {
  type Builder: TemplateBuilder;
  fn builder() -> Self::Builder;
}

pub trait TemplateBuilder: Sized {
  type Target;
  fn build_tml(self) -> Self::Target;
}

pub trait FillTml<M: ImplMarker, C> {
  fn fill(&mut self, c: C);
}

use helper_impl::IntoComposeChildState;
impl<M, T, C, Target> ComposeWithChild<[M; 0], C> for T
where
  T: IntoComposeChildState,
  State<T::C>: ComposeWithChild<M, C, Target = Target>,
{
  type Target = Target;

  fn with_child(self, child: C) -> Self::Target {
    self.into_compose_child_state().with_child(child)
  }
}

mod with_target_child {
  use super::*;
  use crate::widget::SelfImpl;

  impl<C, W, M> ComposeWithChild<[M; 0], C> for State<W>
  where
    W: ComposeChild,
    C: IntoTargetChild<M, W::Child>,
    M: ImplMarker,
  {
    type Target = Widget;

    #[inline]
    fn with_child(self, child: C) -> Self::Target {
      ComposeChild::compose_child(self, child.into_target_child())
    }
  }

  pub trait IntoTargetChild<M: ImplMarker, Child> {
    fn into_target_child(self) -> Child;
  }

  impl<T> IntoTargetChild<SelfImpl, T> for T {
    #[inline]
    fn into_target_child(self) -> T { self }
  }

  impl<M, T> IntoTargetChild<NotSelf<[M; 0]>, Widget> for T
  where
    T: IntoWidget<NotSelf<M>>,
  {
    #[inline]
    fn into_target_child(self) -> Widget { self.into_widget() }
  }

  impl<D, M> IntoTargetChild<NotSelf<[M; 1]>, Widget> for Stateful<DynWidget<Option<D>>>
  where
    D: IntoWidget<M> + 'static,
    M: ImplMarker,
  {
    #[inline]
    fn into_target_child(self) -> Widget { self.into_widget() }
  }

  impl<T, C> IntoTargetChild<NotSelf<[(); 2]>, State<C>> for T
  where
    T: Into<State<C>>,
  {
    #[inline]
    fn into_target_child(self) -> State<C> { self.into() }
  }

  impl<T, C, M> IntoTargetChild<NotSelf<[M; 3]>, Option<C>> for Option<T>
  where
    T: IntoTargetChild<NotSelf<M>, C>,
  {
    #[inline]
    fn into_target_child(self) -> Option<C> { self.map(IntoTargetChild::into_target_child) }
  }

  impl<T, M, C> IntoTargetChild<NotSelf<[M; 4]>, Option<C>> for T
  where
    T: IntoTargetChild<M, C>,
    M: ImplMarker,
  {
    #[inline]
    fn into_target_child(self) -> Option<C> { Some(self.into_target_child()) }
  }

  impl<M1, M2, W, W2, C, C2> IntoTargetChild<NotSelf<[(M1, M2); 4]>, WidgetPair<W2, C2>>
    for WidgetPair<W, C>
  where
    C: IntoTargetChild<NotSelf<M1>, C2>,
    W: IntoTargetChild<NotSelf<M2>, W2>,
  {
    #[inline]
    fn into_target_child(self) -> WidgetPair<W2, C2> {
      let Self { widget, child } = self;
      WidgetPair {
        widget: widget.into_target_child(),
        child: child.into_target_child(),
      }
    }
  }

  impl<M, W, C, C2> IntoTargetChild<NotSelf<[M; 5]>, WidgetPair<W, C2>> for WidgetPair<W, C>
  where
    C: IntoTargetChild<NotSelf<M>, C2>,
  {
    #[inline]
    fn into_target_child(self) -> WidgetPair<W, C2> {
      let Self { widget, child } = self;
      WidgetPair {
        widget,
        child: child.into_target_child(),
      }
    }
  }

  impl<M, W, W2, C> IntoTargetChild<NotSelf<[M; 6]>, WidgetPair<W2, C>> for WidgetPair<W, C>
  where
    W: IntoTargetChild<NotSelf<M>, W2>,
  {
    #[inline]
    fn into_target_child(self) -> WidgetPair<W2, C> {
      let Self { widget, child } = self;
      WidgetPair {
        widget: widget.into_target_child(),
        child,
      }
    }
  }
}

mod with_vec_child {
  use crate::dynamic_widget::DynRender;

  use super::{with_target_child::IntoTargetChild, *};

  impl<C, W, C2, M> ComposeWithChild<[M; 3], C> for State<W>
  where
    W: ComposeChild<Child = Vec<C2>>,
    C: FillVecChild<M, C2>,
  {
    type Target = ComposePair<State<W>, Vec<C2>>;

    fn with_child(self, child: C) -> Self::Target {
      let mut vec = vec![];
      child.fill_in(&mut vec);
      ComposePair { widget: self, child: vec }
    }
  }

  impl<C, W, C2, M> ComposeWithChild<[M; 3], C> for ComposePair<State<W>, Vec<C2>>
  where
    C: FillVecChild<M, C2>,
  {
    type Target = ComposePair<State<W>, Vec<C2>>;

    #[inline]
    fn with_child(mut self, child: C) -> Self::Target {
      child.fill_in(&mut self.child);
      self
    }
  }

  pub trait FillVecChild<M, C> {
    fn fill_in(self, child: &mut Vec<C>);
  }

  impl<T> FillVecChild<[(); 0], T> for T {
    fn fill_in(self, child: &mut Vec<T>) { child.push(self) }
  }

  impl<T, C> FillVecChild<[(); 1], State<C>> for T
  where
    T: Into<State<C>>,
  {
    fn fill_in(self, child: &mut Vec<State<C>>) { child.push(self.into()) }
  }

  impl<M, T> FillVecChild<NotSelf<[M; 2]>, Widget> for T
  where
    T: IntoWidget<NotSelf<M>>,
  {
    fn fill_in(self, child: &mut Vec<Widget>) { child.push(self.into_widget()) }
  }

  impl<D, M> FillVecChild<[M; 3], Widget> for Stateful<DynWidget<D>>
  where
    D: IntoIterator + 'static,
    D::Item: IntoWidget<M>,
    M: ImplMarker,
  {
    fn fill_in(self, children: &mut Vec<Widget>) {
      children.push(DynRender::new(self).into_widget())
    }
  }

  impl<C, M, C2> FillVecChild<[M; 4], C2> for C
  where
    C: IntoIterator,
    C::Item: FillVecChild<M, C2>,
  {
    fn fill_in(self, children: &mut Vec<C2>) { self.into_iter().for_each(|c| c.fill_in(children)) }
  }

  impl<M1, M2, W, W2, C, C2> FillVecChild<NotSelf<[(M1, M2); 5]>, WidgetPair<W2, C2>>
    for WidgetPair<W, C>
  where
    C: IntoTargetChild<NotSelf<M1>, C2>,
    W: IntoTargetChild<NotSelf<M2>, W2>,
  {
    fn fill_in(self, child: &mut Vec<WidgetPair<W2, C2>>) {
      WidgetPair {
        widget: self.widget.into_target_child(),
        child: self.child.into_target_child(),
      }
      .fill_in(child)
    }
  }

  impl<W, C, C2, M> FillVecChild<NotSelf<[M; 6]>, WidgetPair<W, C2>> for WidgetPair<W, C>
  where
    C: IntoTargetChild<NotSelf<M>, C2>,
  {
    fn fill_in(self, child: &mut Vec<WidgetPair<W, C2>>) {
      WidgetPair {
        widget: self.widget,
        child: self.child.into_target_child(),
      }
      .fill_in(child)
    }
  }

  impl<M, W, W2, C> FillVecChild<NotSelf<[M; 7]>, WidgetPair<W2, C>> for WidgetPair<W, C>
  where
    W: IntoTargetChild<NotSelf<M>, W2>,
  {
    fn fill_in(self, child: &mut Vec<WidgetPair<W2, C>>) {
      WidgetPair {
        widget: self.widget.into_target_child(),
        child: self.child,
      }
      .fill_in(child)
    }
  }

  impl<T> FillVecChild<NotSelf<[(); 8]>, T::Target> for T
  where
    T: TemplateBuilder,
  {
    fn fill_in(self, child: &mut Vec<T::Target>) { self.build_tml().fill_in(child) }
  }
}

mod with_child_template {
  use crate::{dynamic_widget::DynRender, prelude::WidgetOf, widget::SelfImpl};

  use super::*;

  macro_rules! impl_template_with_child {
    ($child: ty, $idx:tt) => {
      impl<W, C, Child, Builder, M> ComposeWithChild<[M; $idx], C> for State<W>
      where
        W: ComposeChild<Child = $child>,
        Child: Template<Builder = Builder>,
        Builder: FillTml<M, C>,
        M: ImplMarker,
      {
        type Target = ComposePair<State<W>, Builder>;

        fn with_child(self, c: C) -> Self::Target {
          let mut builder = Child::builder();
          builder.fill(c);
          ComposePair { widget: self, child: builder }
        }
      }
    };
  }

  impl_template_with_child!(Child, 5);
  impl_template_with_child!(Option<Child>, 6);

  impl<W, C, Builder, M> ComposeWithChild<[M; 5], C> for ComposePair<State<W>, Builder>
  where
    W: ComposeChild,
    Builder: FillTml<M, C>,
    M: ImplMarker,
  {
    type Target = ComposePair<State<W>, Builder>;

    fn with_child(mut self, c: C) -> Self::Target {
      self.child.fill(c);
      self
    }
  }

  impl<T, C, M> ComposeWithChild<NotSelf<[M; 6]>, C> for T
  where
    T: TemplateBuilder + FillTml<M, C>,
    M: ImplMarker,
  {
    type Target = Self;

    #[inline]
    fn with_child(mut self, child: C) -> Self::Target {
      self.fill(child);
      self
    }
  }

  macro_rules! impl_fill_widget {
    ($child: ty, $idx: tt  $(,$static:lifetime)?) => {
      impl<C, M, T> FillTml<NotSelf<[M; $idx]>, $child> for T
      where
        C: IntoWidget<NotSelf<M>> $(+$static)?,
        T: FillTml<SelfImpl, Widget>,
      {
        #[inline]
        fn fill(&mut self, c: $child)  { self.fill(c.into_widget()) }
      }
    };
  }

  impl_fill_widget!(C, 0);
  impl_fill_widget!(Stateful<DynWidget<Option<C>>>, 1, 'static);

  macro_rules! impl_fill_tml_for_state {
    ($name: ty, $idx:tt $(,$static:lifetime)?) => {
      impl<T, C $(: $static)?> FillTml<NotSelf<[(); $idx]>, $name> for T
      where
        T: FillTml<SelfImpl, State<C>>,
      {
        #[inline]
        fn fill(&mut self, c: $name) { self.fill(State::<C>::from(c)) }
      }
    };
  }

  impl_fill_tml_for_state!(C, 2);
  impl_fill_tml_for_state!(Stateful<C>, 3);
  impl_fill_tml_for_state!(Stateful<DynWidget<C>>, 4, 'static);

  macro_rules! impl_fill_pair_state_child {
    ($ty: ty, $idx: tt$(,$static:lifetime)?) => {
      impl<W, C, M, T> FillTml<NotSelf<[M; $idx]>, WidgetPair<W, $ty>> for T
      where
        T: FillTml<SelfImpl, WidgetPair<W, State<C>>>,
        $(C: $static)?
      {
        #[inline]
        fn fill(&mut self, c: WidgetPair<W, $ty>){
          let WidgetPair { widget, child } = c;
          self.fill( WidgetPair {
            widget,
            child: State::<C>::from(child),
          })
        }
      }
    };
  }

  impl_fill_pair_state_child!(C, 5);
  impl_fill_pair_state_child!(Stateful<C>, 6);
  impl_fill_pair_state_child!(Stateful<DynWidget<C>>, 7, 'static);

  macro_rules! impl_fill_pair_state_parent {
    ($ty: ty, $idx: tt$(,$static:lifetime)?) => {
      impl<W, C, M, T> FillTml<NotSelf<[M; $idx]>, WidgetPair<$ty, C>> for T
      where
        T: FillTml<SelfImpl, WidgetPair<State<W>, C>>,
        $(W: $static)?
      {
        #[inline]
        fn fill(&mut self, c: WidgetPair<$ty, C>){
          let WidgetPair { widget, child } = c;
          self.fill( WidgetPair {
            widget: State::<W>::from(widget),
            child,
          });
        }
      }
    };
  }

  impl_fill_pair_state_parent!(W, 8);
  impl_fill_pair_state_parent!(Stateful<W>, 9);
  impl_fill_pair_state_parent!(Stateful<DynWidget<W>>, 10, 'static);

  macro_rules! impl_fill_pair_widget {
    ($child: ty, $idx: tt  $(,$static:lifetime)?) => {
      impl<T, W, C, M> FillTml<NotSelf<[M; $idx]>, WidgetPair<W, $child>> for T
      where
        T: FillTml<SelfImpl, WidgetOf<W>>,
        C: IntoWidget<NotSelf<M>>  $(+$static)?,
      {
        #[inline]
        fn fill(&mut self, c: WidgetPair<W, $child>){
          let WidgetPair { widget, child } = c;
          self.fill(WidgetPair { widget, child: child.into_widget() });
        }
      }
    };
  }
  impl_fill_pair_widget!(C, 11);
  impl_fill_pair_widget!(Stateful<DynWidget<Option<C>>>, 12, 'static);

  impl<T, C> FillTml<NotSelf<[(); 13]>, C> for T
  where
    T: FillTml<SelfImpl, Vec<C>>,
  {
    #[inline]
    fn fill(&mut self, c: C) { self.fill(vec![c]) }
  }

  impl<T, C, M> FillTml<NotSelf<[M; 14]>, C> for T
  where
    T: FillTml<SelfImpl, Vec<Widget>>,
    C: IntoWidget<NotSelf<M>>,
  {
    #[inline]
    fn fill(&mut self, c: C) { self.fill(vec![c.into_widget()]) }
  }

  impl<T, D, M> FillTml<NotSelf<[M; 15]>, Stateful<DynWidget<D>>> for T
  where
    T: FillTml<SelfImpl, Vec<Widget>>,
    D: IntoIterator + 'static,
    D::Item: IntoWidget<M>,
    M: ImplMarker,
  {
    #[inline]
    fn fill(&mut self, c: Stateful<DynWidget<D>>) {
      self.fill(vec![DynRender::new(c).into_widget()])
    }
  }

  macro_rules! impl_fill_tml_for_state {
    ($ty: ty, $idx: tt$(,$static:lifetime)?) => {
      impl<T, C> FillTml<NotSelf<[(); $idx]>, $ty> for T
      where
        T: FillTml<SelfImpl, Vec<State<C>>>,
        $(C: $static)?
      {
        #[inline]
        fn fill(&mut self, c: $ty) { self.fill(vec![State::<C>::from(c)]) }
      }
    };
  }

  impl_fill_tml_for_state!(C, 16);
  impl_fill_tml_for_state!(Stateful<C>, 17);
  impl_fill_tml_for_state!(Stateful<DynWidget<C>>, 18, 'static);

  macro_rules! impl_fill_tml_for_vec_state {
    ($ty: ty, $idx: tt$(,$static:lifetime)?) => {
      impl<T, C> FillTml<NotSelf<[(); $idx]>, $ty> for T
      where
        T: FillTml<SelfImpl, Vec<State<C>>>,
        $ty: IntoIterator::<Item = $ty>,
        $(C: $static)?
      {
        fn fill(&mut self, c: $ty) {
          let  vec = c.into_iter().map(State::<C>::from).collect();
          self.fill(vec)
        }
      }
    };
  }

  impl_fill_tml_for_vec_state!(C, 19);
  impl_fill_tml_for_vec_state!(Stateful<C>, 20);
  impl_fill_tml_for_vec_state!(Stateful<DynWidget<C>>, 21, 'static);

  impl<T, C> FillTml<NotSelf<[(); 22]>, C> for T
  where
    T: FillTml<SelfImpl, C::Target>,
    C: TemplateBuilder,
  {
    #[inline]
    fn fill(&mut self, c: C) { self.fill(c.build_tml()) }
  }

  impl<W, T> IntoWidget<NotSelf<[(); 1]>> for ComposePair<State<W>, T>
  where
    W: ComposeChild,
    T: TemplateBuilder<Target = W::Child>,
  {
    #[inline]
    fn into_widget(self) -> Widget {
      let Self { widget, child } = self;
      ComposeChild::compose_child(widget, child.build_tml())
    }
  }

  impl<W, T, C> IntoWidget<NotSelf<[(); 2]>> for ComposePair<State<W>, T>
  where
    W: ComposeChild<Child = Option<C>>,
    T: TemplateBuilder<Target = C>,
  {
    #[inline]
    fn into_widget(self) -> Widget {
      let Self { widget, child } = self;
      ComposeChild::compose_child(widget, Some(child.build_tml()))
    }
  }
}

impl<W, C> IntoWidget<NotSelf<[(); 0]>> for ComposePair<State<W>, C>
where
  W: ComposeChild<Child = C>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    ComposeChild::compose_child(widget, child)
  }
}

mod helper_impl {
  use super::*;
  pub trait IntoComposeChildState {
    type C: ComposeChild;
    fn into_compose_child_state(self) -> State<Self::C>;
  }

  macro_rules! impl_compose_child_state {
    ($ty: ty $(,$static:lifetime)?) => {
      impl<C: ComposeChild $(+ $static)?> IntoComposeChildState for $ty {
        type C = C;
        #[inline]
        fn into_compose_child_state(self) -> State<Self::C> { self.into() }
      }
    };
  }

  impl_compose_child_state!(C);
  impl_compose_child_state!(Stateful<C>);
  impl_compose_child_state!(Stateful<DynWidget<C>>, 'static);
}
