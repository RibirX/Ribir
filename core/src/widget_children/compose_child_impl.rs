use child_convert::SELF;

use super::*;
// Template use to construct child of a widget.
pub trait Template: Sized {
  type Builder: TemplateBuilder;
  fn builder() -> Self::Builder;
}

pub trait TemplateBuilder: Sized {
  type Target;
  fn build_tml(self) -> Self::Target;
}

impl<T, C, Child, const M: usize> WithChild<C, 2, M> for T
where
  T: StateWriter,
  T::Value: ComposeChild<Child = Child>,
  C: IntoChild<Child, M>,
{
  type Target = Pair<T, Child>;

  #[track_caller]
  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    Pair { parent: self, child: c.into_child(ctx) }
  }
}

stateless_with_child!(3);

impl<P, T, C, const M: usize> WithChild<C, 4, M> for P
where
  P: StateWriter,
  P::Value: ComposeChild<Child = T>,
  T: Template,
  T::Builder: WithChild<C, 2, M, Target = T::Builder>,
{
  type Target = Pair<Self, T::Builder>;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let child = T::builder().with_child(child, ctx);
    Pair { parent: self, child }
  }
}

stateless_with_child!(5);

impl<W, C, T, const M: usize> WithChild<C, 6, M> for W
where
  W: StateWriter,
  W::Value: ComposeChild<Child = Option<T>>,
  T: Template,
  T::Builder: WithChild<C, 2, M, Target = T::Builder>,
{
  type Target = Pair<W, T::Builder>;
  #[track_caller]
  fn with_child(self, c: C, ctx: &BuildCtx) -> Self::Target {
    let builder = T::builder();
    let child = builder.with_child(c, ctx);
    Pair { parent: self, child }
  }
}
stateless_with_child!(7);

fat_obj_with_child!(2, 3, 4, 5, 6, 7);

impl<const M: usize, W, const N: usize, C1, C2> WithChild<C2, N, M> for Pair<W, C1>
where
  C1: WithChild<C2, N, M>,
{
  type Target = Pair<W, C1::Target>;
  #[track_caller]
  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair { parent: widget, child: child.with_child(c, ctx) }
  }
}

impl<W, C> IntoWidgetStrict<FN> for Pair<W, C>
where
  W: StateWriter,
  W::Value: ComposeChild,
  C: IntoChild<<W::Value as ComposeChild>::Child, 0>,
{
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    let Self { parent, child } = self;
    ComposeChild::compose_child(parent, child.into_child(ctx)).into_widget(ctx)
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

macro_rules! vec_with_child {
  ($($m:ident),*) => {
    $(
      impl<C, T> WithChild<C, 2, $m> for Vec<T>
      where
        C: IntoChild<T, $m>,
      {
        type Target = Self;

        #[inline]
        #[track_caller]
        fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
          self.push(child.into_child(ctx));
          self
        }
      }
    )*
  };
}

macro_rules! vec_with_iter_child {
  ($($m:ident),*) => {
    $(
      impl<C, T> WithChild<C, 2, {5 + $m}> for Vec<T>
      where
        C: IntoIterator,
        C::Item: IntoChild<T, $m>,
      {
        type Target = Self;

        #[inline]
        #[track_caller]
        fn with_child(mut self, child: C, ctx: &BuildCtx) -> Self::Target {
          self.extend(child.into_iter().map(|v| v.into_child(ctx)));
          self
        }
      }
    )*
  };
}
vec_with_child!(SELF, COMPOSE, RENDER, FN);
vec_with_iter_child!(SELF, COMPOSE, RENDER, FN);

// todo: remove it, keep it for backward compatibility.
impl<W, C, const M: usize> WithChild<C, 8, M> for State<W>
where
  W: ComposeDecorator + 'static,
  C: IntoWidget<M>,
{
  type Target = Widget;

  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Widget {
    let tid = TypeId::of::<W>();
    let style = ctx.find_cfg(|t| match t {
      Theme::Full(t) => t.compose_decorators.styles.get(&tid),
      Theme::Inherit(i) => i
        .compose_decorators
        .as_ref()
        .and_then(|s| s.styles.get(&tid)),
    });

    let host = child.into_widget(ctx);
    if let Some(style) = style {
      style(Box::new(self), host, ctx)
    } else {
      ComposeDecorator::compose_decorator(self, host).into_widget(ctx)
    }
  }
}

impl<T: 'static, C, const M: usize> WithChild<C, 9, M> for T
where
  T: ComposeDecorator,
  C: IntoWidget<M>,
{
  type Target = Widget;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::value(self).with_child(child, ctx)
  }
}

fat_obj_with_child!(8, 9);

macro_rules! stateless_with_child {
  ($n:literal) => {
    impl<P, C, const M: usize> WithChild<C, $n, M> for P
    where
      P: ComposeChild,
      State<P>: WithChild<C, { $n - 1 }, M>,
    {
      type Target = <State<P> as WithChild<C, { $n - 1 }, M>>::Target;

      fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
        State::value(self).with_child(child, ctx)
      }
    }
  };
}

macro_rules! fat_obj_with_child {
  ($($n:literal),*) => {
    $(
      impl<W, C, const M: usize> WithChild<C, $n, M> for FatObj<W>
      where
        W: WithChild<C, $n, M>,
      {
        type Target = FatObj<W::Target>;

        fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
          self.map(|host| host.with_child(child, ctx))
        }
      }
    )*
  };
}

use fat_obj_with_child;
use stateless_with_child;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::MockBox;

  #[derive(Template)]
  enum PTml {
    Void(Void),
  }

  struct P;

  impl ComposeChild for P {
    type Child = PTml;
    fn compose_child(
      _: impl StateWriter<Value = Self>, _: Self::Child,
    ) -> impl IntoWidgetStrict<FN> {
      fn_widget!(Void)
    }
  }

  #[derive(Declare)]
  struct X;

  impl ComposeChild for X {
    type Child = Widget;

    fn compose_child(
      _: impl StateWriter<Value = Self>, _: Self::Child,
    ) -> impl IntoWidgetStrict<FN> {
      fn_widget!(Void)
    }
  }

  #[test]
  fn template_fill_template() { let _ = |ctx| P.with_child(Void, ctx).into_widget(ctx); }

  #[test]
  fn pair_compose_child() {
    let _ = |ctx| -> Widget {
      MockBox { size: ZERO_SIZE }
        .with_child(X.with_child(Void {}, ctx), ctx)
        .into_widget(ctx)
    };
  }

  #[derive(Declare)]
  struct PipeParent;

  impl ComposeChild for PipeParent {
    type Child = BoxPipe<usize>;

    fn compose_child(
      _: impl StateWriter<Value = Self>, _: Self::Child,
    ) -> impl IntoWidgetStrict<FN> {
      fn_widget!(Void)
    }
  }

  #[test]
  fn compose_pipe_child() {
    let _value_child = fn_widget! {
      @PipeParent {  @ { BoxPipe::value(0) } }
    };

    let _pipe_child = fn_widget! {
      let state = State::value(0);
      @PipeParent {  @ { pipe!(*$state) } }
    };
  }
}
