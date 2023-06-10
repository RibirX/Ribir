use crate::{
  dynamic_widget::DynWidget,
  state::{State, Stateful},
  widget::{ImplMarker, IntoWidget, NotSelf, Widget},
};

use super::{
  child_convert::{FillVec, IntoChild},
  ComposeChild, WidgetPair,
};

/// Trait specify what child a compose child widget can have, and the target
/// type after widget compose its child.
pub trait ComposeWithChild<M, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

/// The pair a `ComposeChild` widget with its child that may some children not
/// fill.
pub struct ComposePair<W, C> {
  pub widget: W,
  pub child: C,
}
/// `DecorateTml` lets a template can declare like a widget, so a template can
/// support built-in widgets. For example, if you define a template `Leading`,
/// you can use DecorateTml<Leading> as the template, so the user can use
/// built-in widgets for `Leading`.
pub struct DecorateTml<T: TmlFlag, C> {
  pub(crate) decorator: Box<dyn FnOnce(Widget) -> Widget>,
  pub(crate) tml_flag: T,
  pub(crate) child: C,
}

/// Trait mark a type is a template flag that can be used with `DecorateTml`.
pub trait TmlFlag {}

// Template use to construct child of a widget.
pub trait Template: Sized {
  type Builder: TemplateBuilder;
  fn builder() -> Self::Builder;
}

pub trait TemplateBuilder: Sized {
  type Target;
  fn build_tml(self) -> Self::Target;
}

pub trait FillTml<M, C> {
  fn fill_tml(&mut self, c: C);
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

impl<M, W, C1, C2> ComposeWithChild<[M; 0], C2> for WidgetPair<W, C1>
where
  C1: ComposeWithChild<M, C2>,
{
  type Target = WidgetPair<W, C1::Target>;

  fn with_child(self, c: C2) -> Self::Target {
    let WidgetPair { widget, child } = self;
    WidgetPair { widget, child: child.with_child(c) }
  }
}

impl<C, W, M> ComposeWithChild<[M; 0], C> for State<W>
where
  W: ComposeChild,
  C: IntoChild<M, W::Child>,
  M: ImplMarker,
{
  type Target = ComposePair<State<W>, W::Child>;

  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    ComposePair {
      widget: self,
      child: child.into_child(),
    }
  }
}

impl<C, W, Child, M> ComposeWithChild<[M; 1], C> for State<W>
where
  W: ComposeChild<Child = Vec<Child>>,
  C: FillVec<M, Child>,
  M: ImplMarker,
{
  type Target = ComposePair<State<W>, Vec<Child>>;

  #[inline]
  fn with_child(self, c: C) -> Self::Target {
    let mut child = vec![];
    c.fill_vec(&mut child);
    ComposePair { widget: self, child }
  }
}

impl<W, C1, C2, M> ComposeWithChild<[M; 2], C1> for ComposePair<State<W>, Vec<C2>>
where
  C1: FillVec<M, C2>,
{
  type Target = ComposePair<State<W>, Vec<C2>>;

  fn with_child(mut self, child: C1) -> Self::Target {
    child.fill_vec(&mut self.child);
    self
  }
}

mod more_impl_for_fill_tml {
  use super::*;
  macro_rules! impl_template_with_child {
    ($child: ty, $idx:tt) => {
      impl<W, C, Child, M> ComposeWithChild<[M; $idx], C> for State<W>
      where
        W: ComposeChild<Child = $child>,
        Child: Template,
        Child::Builder: FillTml<M, C>,
      {
        type Target = ComposePair<State<W>, Child::Builder>;

        fn with_child(self, c: C) -> Self::Target {
          let mut builder = Child::builder();
          builder.fill_tml(c);
          ComposePair { widget: self, child: builder }
        }
      }
    };
  }

  impl_template_with_child!(Child, 5);
  impl_template_with_child!(Option<Child>, 6);

  impl<W, C, Builder, M> ComposeWithChild<[M; 6], C> for ComposePair<State<W>, Builder>
  where
    W: ComposeChild,
    Builder: FillTml<M, C>,
  {
    type Target = ComposePair<State<W>, Builder>;

    fn with_child(mut self, c: C) -> Self::Target {
      self.child.fill_tml(c);
      self
    }
  }

  impl<T, C, M> ComposeWithChild<NotSelf<[M; 7]>, C> for T
  where
    T: TemplateBuilder + FillTml<M, C>,
  {
    type Target = Self;

    #[inline]
    fn with_child(mut self, child: C) -> Self::Target {
      self.fill_tml(child);
      self
    }
  }
}

mod decorate_tml_impl {
  use super::*;
  use crate::widget::SelfImpl;

  impl<M, W, C> ComposeWithChild<NotSelf<[M; 0]>, C> for State<W>
  where
    W: ComposeChild<Child = Widget> + 'static,
    C: DecorateTmlMarker<M>,
    M: ImplMarker,
  {
    type Target = ComposePair<Self, C>;

    #[inline]
    fn with_child(self, child: C) -> Self::Target { ComposePair { widget: self, child } }
  }

  impl<T: TmlFlag, C> DecorateTml<T, C> {
    pub fn decorate<M: ImplMarker, W: IntoWidget<M>>(
      self,
      tml_to_widget: impl FnOnce(T, C) -> W,
    ) -> Widget {
      let Self { decorator, tml_flag, child } = self;
      decorator(tml_to_widget(tml_flag, child).into_widget())
    }
  }

  trait DecorateTmlMarker<M: ImplMarker> {}

  impl<T: TmlFlag, C> DecorateTmlMarker<SelfImpl> for DecorateTml<T, C> {}

  impl<T: TmlFlag, C> DecorateTmlMarker<NotSelf<()>> for WidgetPair<T, C> {}

  impl<M, T, C> DecorateTmlMarker<NotSelf<M>> for WidgetPair<T, C>
  where
    M: ImplMarker,
    C: DecorateTmlMarker<M>,
  {
  }

  impl<M: ImplMarker, T, C: DecorateTmlMarker<M>> DecorateTmlMarker<M> for ComposePair<T, C> {}
}

impl<W, M, C> IntoWidget<NotSelf<[M; 0]>> for ComposePair<State<W>, C>
where
  W: ComposeChild,
  C: IntoChild<M, W::Child>,
  M: ImplMarker,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    ComposeChild::compose_child(widget, child.into_child())
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

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use super::*;
  use crate::{prelude::*, test_helper::MockBox};
  #[derive(Template)]
  struct PTml {
    #[template(flat_fill)]
    _child: CTml,
  }

  #[derive(Template)]
  enum CTml {
    Void(Void),
  }

  struct P;

  impl ComposeChild for P {
    type Child = PTml;
    fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
  }

  #[derive(Declare)]
  struct X;

  impl ComposeChild for X {
    type Child = Widget;

    fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
  }

  #[test]
  fn template_fill_template() { let _ = P.with_child(Void).into_widget(); }

  #[test]
  fn pair_compose_child() {
    let _ = MockBox { size: ZERO_SIZE }
      .with_child(X)
      .with_child(Void {})
      .into_widget();
  }

  #[test]
  fn enum_widget_compose_child() {
    let flag = true;
    let _ = widget! {
      DynWidget {
        dyns: match flag {
          true => WidgetE2::A(MockBox{ size: ZERO_SIZE }.with_child(X)),
          false => WidgetE2::B(X),
        },
        X { Void {} }
      }
    };
  }

  #[derive(SingleChild)]
  struct Tml;
  struct A;
  impl TmlFlag for Tml {}

  #[test]
  fn decorate_tml() {
    struct WithDecorate;

    impl ComposeChild for WithDecorate {
      type Child = DecorateTml<Tml, A>;

      fn compose_child(_this: State<Self>, child: Self::Child) -> Widget {
        child.decorate(|_, _| Void.into_widget())
      }
    }
    let mb = MockBox { size: Size::zero() };
    let _: Widget = WithDecorate
      .with_child(mb.clone().with_child(mb.with_child(Tml.with_child(A))))
      .into_widget();
    let _: Widget = WithDecorate.with_child(Tml.with_child(A)).into_widget();
  }

  #[test]
  fn with_embed_decorate() {
    struct WithDecorate;
    #[derive(Template)]
    struct EmbedDecorateTml(DecorateTml<Tml, A>);

    impl ComposeChild for WithDecorate {
      type Child = EmbedDecorateTml;

      fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
        child.0.decorate(|_, _| Void.into_widget())
      }
    }

    let _ = WithDecorate.with_child(Tml.with_child(A)).into_widget();
    let mb = MockBox { size: Size::zero() };
    let _ = WithDecorate.with_child(mb.clone().with_child(Tml.with_child(A)));
    let cursor = Cursor {
      cursor: Rc::new(Cell::new(CursorIcon::Hand)),
    };
    let x = cursor.with_child(Tml.with_child(A));
    let _ = WithDecorate.with_child(mb.with_child(x)).into_widget();
  }
}
