use super::{ComposeChild, Pair};
use crate::{
  context::BuildCtx,
  prelude::{BoxPipe, ChildFrom},
  state::{State, StateWriter},
  widget::{Widget, WidgetBuilder},
};

/// Trait specify what child a compose child widget can have, and the target
/// type after widget compose its child.
pub trait ComposeWithChild<C, M> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
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

impl<M, T, C> ComposeWithChild<C, [M; 0]> for T
where
  T: ComposeChild,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::value(self).with_child(child, ctx)
  }
}

impl<M, W, C1, C2> ComposeWithChild<C2, M> for Pair<W, C1>
where
  C1: ComposeWithChild<C2, M>,
{
  type Target = Pair<W, C1::Target>;
  #[track_caller]
  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair { parent: widget, child: child.with_child(c, ctx) }
  }
}

impl<C, W, M, Child> ComposeWithChild<C, [M; 1]> for W
where
  W: StateWriter,
  W::Value: ComposeChild<Child = Child>,
  Child: ChildFrom<C, M>,
{
  type Target = Pair<Self, Child>;

  #[inline]
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    Pair { parent: self, child: ChildFrom::child_from(child, ctx) }
  }
}

impl<W, C, Child, M> ComposeWithChild<C, [M; 2]> for W
where
  W: StateWriter,
  W::Value: ComposeChild<Child = Child>,
  Child: Template,
  Child::Builder: ComposeWithChild<C, M, Target = Child::Builder>,
{
  type Target = Pair<W, Child::Builder>;

  #[inline]
  #[track_caller]
  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let builder = Child::builder();
    let child = builder.with_child(c, ctx);
    Pair { parent: self, child }
  }
}

impl<W, C, Child, M> ComposeWithChild<C, [M; 3]> for W
where
  W: StateWriter,
  W::Value: ComposeChild<Child = Option<Child>>,
  Child: Template,
  Child::Builder: ComposeWithChild<C, M, Target = Child::Builder>,
{
  type Target = Pair<W, Child::Builder>;
  #[track_caller]
  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let builder = Child::builder();
    let child = builder.with_child(c, ctx);
    Pair { parent: self, child }
  }
}

impl<W, C, Child> WidgetBuilder for Pair<W, C>
where
  W: StateWriter,
  W::Value: ComposeChild<Child = Child>,
  Child: From<C>,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget {
    let Self { parent, child } = self;
    ComposeChild::compose_child(parent, child.into()).build(ctx)
  }
}

impl<T: 'static> From<T> for BoxPipe<T> {
  #[inline]
  fn from(t: T) -> Self { BoxPipe::value(t) }
}

// impl Vec<T> as Template

impl<T> Template for Vec<T> {
  type Builder = Self;
  #[inline]
  fn builder() -> Self::Builder { vec![] }
}

impl<T> Template for BoxPipe<Vec<T>> {
  type Builder = Vec<T>;
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
  #[track_caller]
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
  #[track_caller]
  fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
    self.extend(
      child
        .into_iter()
        .map(|v| ChildFrom::child_from(v, ctx)),
    );
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
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
      fn_widget!(Void)
    }
  }

  #[derive(Declare)]
  struct X;

  impl ComposeChild for X {
    type Child = Widget;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
      fn_widget!(Void)
    }
  }

  #[test]
  fn template_fill_template() { let _ = |ctx| P.with_child(Void, ctx).build(ctx); }

  #[test]
  fn pair_compose_child() {
    let _ = |ctx| -> Widget {
      MockBox { size: ZERO_SIZE }
        .with_child(X.with_child(Void {}, ctx), ctx)
        .build(ctx)
    };
  }

  #[derive(Declare)]
  struct PipeParent;

  impl ComposeChild for PipeParent {
    type Child = BoxPipe<usize>;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
      fn_widget!(Void)
    }
  }

  #[test]
  fn compose_pipe_child() {
    let _value_child = fn_widget! {
      @PipeParent {  @ { 0 } }
    };

    let _pipe_child = fn_widget! {
      let state = State::value(0);
      @PipeParent {  @ { pipe!(*$state) } }
    };
  }
}
