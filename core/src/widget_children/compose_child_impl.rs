use crate::{
  context::BuildCtx,
  dynamic_widget::DynWidget,
  prelude::ChildFrom,
  state::{State, Stateful},
  widget::{Widget, WidgetBuilder, WidgetId},
};

use super::{child_convert::FillVec, ComposeChild, SinglePair};

/// Trait specify what child a compose child widget can have, and the target
/// type after widget compose its child.
pub trait ComposeWithChild<C, M> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
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

impl<M, T, C> ComposeWithChild<C, M> for T
where
  T: ComposeChild,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::Stateless(self).with_child(child, ctx)
  }
}

impl<M, T, C> ComposeWithChild<C, M> for Stateful<T>
where
  T: ComposeChild,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::Stateful(self).with_child(child, ctx)
  }
}

impl<M, T, C> ComposeWithChild<C, M> for Stateful<DynWidget<T>>
where
  T: ComposeChild + 'static,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::from(self).with_child(child, ctx)
  }
}

impl<M, W, C1, C2> ComposeWithChild<C2, M> for SinglePair<W, C1>
where
  C1: ComposeWithChild<C2, M>,
{
  type Target = SinglePair<W, C1::Target>;

  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let SinglePair { widget, child } = self;
    SinglePair {
      widget,
      child: child.with_child(c, ctx),
    }
  }
}

impl<W, C, Builder, M> ComposeWithChild<C, M> for ComposePair<State<W>, Builder>
where
  W: ComposeChild,
  Builder: TemplateBuilder + ComposeWithChild<C, M, Target = Builder>,
{
  type Target = ComposePair<State<W>, Builder>;

  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let Self { widget, child } = self;
    let child = child.with_child(c, ctx);
    ComposePair { widget, child }
  }
}

impl<C, W, M> ComposeWithChild<C, [M; 0]> for State<W>
where
  W: ComposeChild,
  W::Child: ChildFrom<C, M>,
{
  type Target = ComposePair<State<W>, W::Child>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target {
    ComposePair {
      widget: self,
      child: ChildFrom::child_from(child),
    }
  }
}

impl<W, C, Child, M> ComposeWithChild<C, [M; 1]> for State<W>
where
  W: ComposeChild<Child = Child>,
  Child: Template,
  Child::Builder: ComposeWithChild<C, M, Target = Child::Builder>,
{
  type Target = ComposePair<State<W>, Child::Builder>;

  #[inline]
  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let builder = W::Child::builder();
    let child = builder.with_child(c, ctx);
    ComposePair { widget: self, child }
  }
}

impl<W, C, Child, M> ComposeWithChild<C, [M; 2]> for State<W>
where
  W: ComposeChild<Child = Option<Child>>,
  Child: Template,
  Child::Builder: ComposeWithChild<C, M, Target = Child::Builder>,
{
  type Target = ComposePair<State<W>, Child::Builder>;

  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let builder = Child::builder();
    let child = builder.with_child(c, ctx);
    ComposePair { widget: self, child }
  }
}

pub(crate) mod decorate_tml_impl {
  use super::*;

  impl<M, W, C> ComposeWithChild<C, [M; 3]> for State<W>
  where
    W: ComposeChild<Child = Widget> + 'static,
    C: DecorateTmlMarker<M>,
  {
    type Target = ComposePair<Self, C>;

    #[inline]
    fn with_child(self, child: C, _: &BuildCtx) -> Self::Target {
      ComposePair { widget: self, child }
    }
  }

  impl<T: TmlFlag, C> DecorateTml<T, C> {
    pub fn decorate(self, tml_to_widget: impl FnOnce(T, C) -> Widget) -> Widget {
      let Self { decorator, tml_flag, child } = self;
      let w = tml_to_widget(tml_flag, child);
      decorator(w)
    }
  }

  trait DecorateTmlMarker<M> {}

  impl<T: TmlFlag, C> DecorateTmlMarker<()> for DecorateTml<T, C> {}

  impl<T: TmlFlag, C> DecorateTmlMarker<[(); 0]> for SinglePair<T, C> {}

  impl<M, T, C: DecorateTmlMarker<M>> DecorateTmlMarker<[M; 1]> for SinglePair<T, C> {}

  impl<M, T, C: DecorateTmlMarker<M>> DecorateTmlMarker<M> for ComposePair<T, C> {}

  pub(crate) trait IntoDecorateTml<C, M> {
    type Flag: TmlFlag;
    fn into_decorate_tml(self) -> DecorateTml<Self::Flag, C>;
  }

  impl<W, C, M, C2> IntoDecorateTml<C, [M; 0]> for SinglePair<W, C2>
  where
    W: TmlFlag,
    C: ChildFrom<C2, M>,
  {
    type Flag = W;

    fn into_decorate_tml(self) -> DecorateTml<Self::Flag, C> {
      let SinglePair { widget: tml_flag, child } = self;
      let decorator = Box::new(|w| w);
      let child = C::child_from(child);
      DecorateTml { decorator, tml_flag, child }
    }
  }

  impl<W, C, C2, M> IntoDecorateTml<C, [M; 1]> for SinglePair<W, C2>
  where
    W: 'static,
    C2: IntoDecorateTml<C, M>,
    SinglePair<W, Widget>: WidgetBuilder,
  {
    type Flag = C2::Flag;

    fn into_decorate_tml(self) -> DecorateTml<Self::Flag, C> {
      let SinglePair { widget, child } = self;
      let DecorateTml { decorator, tml_flag, child } = ChildFrom::child_from(child);
      DecorateTml {
        decorator: Box::new(move |w| SinglePair { widget, child: decorator(w) }.into()),
        tml_flag,
        child,
      }
    }
  }

  impl<W, C, C2, M> IntoDecorateTml<C, M> for ComposePair<State<W>, C2>
  where
    W: ComposeChild<Child = Widget> + 'static,
    C2: IntoDecorateTml<C, M>,
  {
    type Flag = C2::Flag;

    fn into_decorate_tml(self) -> DecorateTml<Self::Flag, C> {
      let ComposePair { widget, child } = self;
      let DecorateTml { decorator, tml_flag, child } = child.into_decorate_tml();
      DecorateTml {
        decorator: Box::new(move |w| ComposeChild::compose_child(widget, decorator(w))),
        tml_flag,
        child,
      }
    }
  }
}

impl<W, C> WidgetBuilder for ComposePair<State<W>, C>
where
  W: ComposeChild,
  W::Child: From<C>,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    ComposeChild::compose_child(widget, child.into()).build(ctx)
  }
}

// impl Vec<T> as Template

impl<T> Template for Vec<T> {
  type Builder = Self;
  #[inline]
  fn builder() -> Self::Builder { vec![] }
}

impl<T> TemplateBuilder for Vec<T> {
  type Target = Self;
  #[inline]
  fn build_tml(self) -> Self::Target { self }
}

impl<M, C, T> ComposeWithChild<C, M> for Vec<T>
where
  C: FillVec<T, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C, _: &BuildCtx) -> Self::Target {
    child.fill_vec(&mut self);
    self
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use super::*;
  use crate::{prelude::*, test_helper::MockBox};
  #[derive(Template)]
  struct PTml {
    _child: CTml,
  }

  #[derive(Template)]
  enum CTml {
    Void(Void),
  }

  struct P;

  impl ComposeChild for P {
    type Child = PTml;
    fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
  }

  #[derive(Declare)]
  struct X;

  impl ComposeChild for X {
    type Child = Widget;

    fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
  }

  #[test]
  fn template_fill_template() { let _ = FnWidget::new(|ctx| P.with_child(Void, ctx).build(ctx)); }

  #[test]
  fn pair_compose_child() {
    let _ = FnWidget::new(|ctx| {
      MockBox { size: ZERO_SIZE }
        .with_child(X, ctx)
        .with_child(Void {}, ctx)
        .build(ctx)
    });
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
        child.decorate(|_, _| Void.into())
      }
    }
    let mb = MockBox { size: Size::zero() };
    let _ = FnWidget::new(|ctx| {
      WithDecorate
        .with_child(
          mb.clone()
            .with_child(mb.with_child(Tml.with_child(A, ctx), ctx), ctx),
          ctx,
        )
        .build(ctx)
    });

    let _ = FnWidget::new(|ctx| {
      WithDecorate
        .with_child(Tml.with_child(A, ctx), ctx)
        .build(ctx)
    });
  }

  #[test]
  fn with_embed_decorate() {
    struct WithDecorate;
    #[derive(Template)]
    struct EmbedDecorateTml(DecorateTml<Tml, A>);

    impl ComposeChild for WithDecorate {
      type Child = EmbedDecorateTml;

      fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
        child.0.decorate(|_, _| Void.into())
      }
    }

    let _ = FnWidget::new(|ctx| {
      WithDecorate
        .with_child(Tml.with_child(A, ctx), ctx)
        .build(ctx)
    });
    let mb = MockBox { size: Size::zero() };
    let _ = FnWidget::new(|ctx| {
      WithDecorate
        .with_child(mb.clone().with_child(Tml.with_child(A, ctx), ctx), ctx)
        .build(ctx)
    });
    let _ = FnWidget::new(|ctx| {
      let cursor = Cursor {
        cursor: Rc::new(Cell::new(CursorIcon::Hand)),
      };
      let x = cursor.with_child(Tml.with_child(A, ctx), ctx);
      WithDecorate
        .with_child(mb.with_child(x, ctx), ctx)
        .build(ctx)
    });
  }
}
