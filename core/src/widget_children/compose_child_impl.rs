use crate::{
  context::BuildCtx,
  prelude::ChildFrom,
  state::{State, Stateful},
  widget::{StrictBuilder, WidgetId},
};

use super::{ComposeChild, SinglePair};

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
    State::value(self).with_child(child, ctx)
  }
}

impl<M, T, C> ComposeWithChild<C, M> for Stateful<T>
where
  T: ComposeChild,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::stateful(self).with_child(child, ctx)
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
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    ComposePair {
      widget: self,
      child: ChildFrom::child_from(child, ctx),
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

impl<W, C> StrictBuilder for ComposePair<State<W>, C>
where
  W: ComposeChild,
  W::Child: From<C>,
{
  #[inline]
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
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

impl<M, C, T> ComposeWithChild<C, [M; 0]> for Vec<T>
where
  T: ChildFrom<C, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    self.push(ChildFrom::child_from(child, ctx));
    self
  }
}

impl<M, C, T> ComposeWithChild<C, [M; 1]> for Vec<T>
where
  C: IntoIterator,
  T: ChildFrom<C::Item, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    self.extend(child.into_iter().map(|v| ChildFrom::child_from(v, ctx)));
    self
  }
}

#[cfg(test)]
mod tests {

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

  #[derive(Declare2)]
  struct X;

  impl ComposeChild for X {
    type Child = Widget;

    fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
  }

  #[test]
  fn template_fill_template() {
    let _ = FnWidget::new(|ctx| P.with_child(Void, ctx).strict_build(ctx));
  }

  #[test]
  fn pair_compose_child() {
    let _ = FnWidget::new(|ctx| {
      MockBox { size: ZERO_SIZE }
        .with_child(X, ctx)
        .with_child(Void {}, ctx)
        .strict_build(ctx)
    });
  }
}
