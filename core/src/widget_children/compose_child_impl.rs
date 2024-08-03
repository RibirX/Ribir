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

impl<'w, T, C, const M: usize> WithChild<'w, C, 2, M> for T
where
  T: StateWriter + 'static,
  T::Value: ComposeChild<'w>,
  C: IntoChild<<T::Value as ComposeChild<'w>>::Child, M> + 'w,
{
  type Target = Pair<T, <T::Value as ComposeChild<'w>>::Child>;

  #[inline]
  fn with_child(self, c: C) -> Self::Target { Pair { parent: self, child: c.into_child() } }
}

stateless_with_child!(3);

impl<'w, P, T, C, const M: usize> WithChild<'w, C, 4, M> for P
where
  P: StateWriter + 'static,
  P::Value: ComposeChild<'w, Child = T>,
  T: Template,
  T::Builder: WithChild<'w, C, 2, M>,
{
  type Target = Pair<Self, <T::Builder as WithChild<'w, C, 2, M>>::Target>;

  fn with_child(self, child: C) -> Self::Target {
    let child = T::builder().with_child(child);
    Pair { parent: self, child }
  }
}

stateless_with_child!(5);

impl<'w, W, C, T, const M: usize> WithChild<'w, C, 6, M> for W
where
  W: StateWriter + 'static,
  W::Value: ComposeChild<'w, Child = Option<T>>,
  T: Template,
  T::Builder: WithChild<'w, C, 2, M>,
{
  type Target = Pair<Self, <T::Builder as WithChild<'w, C, 2, M>>::Target>;
  fn with_child(self, c: C) -> Self::Target {
    let builder = T::builder();
    let child = builder.with_child(c);
    Pair { parent: self, child }
  }
}
stateless_with_child!(7);

fat_obj_with_child!(2, 3, 4, 5, 6, 7);

impl<'w, const M: usize, W: 'w, const N: usize, C1, C2: 'w> WithChild<'w, C2, N, M> for Pair<W, C1>
where
  C1: WithChild<'w, C2, N, M>,
{
  type Target = Pair<W, C1::Target>;

  fn with_child(self, c: C2) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair { parent: widget, child: child.with_child(c) }
  }
}

impl<'w, W, C> IntoWidgetStrict<'w, FN> for Pair<W, C>
where
  W: StateWriter + 'static,
  W::Value: ComposeChild<'w>,
  C: IntoChild<<W::Value as ComposeChild<'w>>::Child, 0> + 'w,
{
  #[inline]
  fn into_widget_strict(self) -> Widget<'w> {
    let Self { parent, child } = self;
    ComposeChild::compose_child(parent, child.into_child()).into_widget()
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
      impl<'w, C:'w, T:'w> WithChild<'w, C, 2, $m> for Vec<T>
      where
        C: IntoChild<T, $m>,
      {
        type Target = Self;

        #[inline]
        fn with_child<>(mut self, child: C) -> Self::Target {
          self.push(child.into_child());
          self
        }
      }
    )*
  };
}

macro_rules! vec_with_iter_child {
  ($($m:ident),*) => {
    $(
      impl<'w, C:'w, T:'w> WithChild<'w, C, 2, {5 + $m}> for Vec<T>
      where
        C: IntoIterator,
        C::Item: IntoChild<T, $m>,
      {
        type Target = Self;

        #[inline]
        fn with_child(mut self, child: C) -> Self::Target {
          self.extend(child.into_iter().map(|v| v.into_child()));
          self
        }
      }
    )*
  };
}
vec_with_child!(SELF, COMPOSE, RENDER, FN);
vec_with_iter_child!(SELF, COMPOSE, RENDER, FN);

// todo: remove it, keep it for backward compatibility.
impl<'w, W, C, const M: usize> WithChild<'w, C, 8, M> for State<W>
where
  W: ComposeDecorator + 'static,
  C: IntoWidget<'w, M>,
{
  type Target = Widget<'w>;

  fn with_child(self, child: C) -> Self::Target {
    let f = move |ctx: &mut BuildCtx| {
      let tid = TypeId::of::<W>();
      let style = ctx.find_cfg(|t| match t {
        Theme::Full(t) => t.compose_decorators.styles.get(&tid),
        Theme::Inherit(i) => i
          .compose_decorators
          .as_ref()
          .and_then(|s| s.styles.get(&tid)),
      });

      let host = child.into_widget();
      if let Some(style) = style {
        style(Box::new(self), host, ctx)
      } else {
        ComposeDecorator::compose_decorator(self, host).into_widget()
      }
    };
    f.into_widget()
  }
}

impl<'w, T, C, const M: usize> WithChild<'w, C, 9, M> for T
where
  T: ComposeDecorator + 'static,
  C: IntoWidget<'w, M>,
{
  type Target = Widget<'w>;

  fn with_child(self, child: C) -> Self::Target { State::value(self).with_child(child) }
}

fat_obj_with_child!(8, 9);

macro_rules! stateless_with_child {
  ($n:literal) => {
    impl<'w, P, C: 'w, const M: usize> WithChild<'w, C, $n, M> for P
    where
      P: ComposeChild<'w>,
      State<P>: WithChild<'w, C, { $n - 1 }, M>,
    {
      type Target = <State<P> as WithChild<'w, C, { $n - 1 }, M>>::Target;

      fn with_child(self, child: C) -> Self::Target { State::value(self).with_child(child) }
    }
  };
}

macro_rules! fat_obj_with_child {
  ($($n:literal),*) => {
    $(
      impl<'w, W, C: 'w, const M: usize> WithChild<'w, C, $n, M> for FatObj<W>
      where
        W: WithChild<'w, C, $n, M>,
      {
        type Target = FatObj<W::Target>;

        fn with_child(self, child: C) -> Self::Target {
          self.map(|host| host.with_child(child))
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

  impl ComposeChild<'static> for P {
    type Child = PTml;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      Void.into_widget()
    }
  }

  #[derive(Declare)]
  struct X;

  impl<'c> ComposeChild<'c> for X {
    type Child = Widget<'c>;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
      Void.into_widget()
    }
  }

  #[test]
  fn template_fill_template() { let _ = |_: &BuildCtx| P.with_child(Void).into_widget(); }

  #[test]
  fn pair_compose_child() {
    let _ = |_: &BuildCtx| -> Widget {
      MockBox { size: ZERO_SIZE }
        .with_child(X.with_child(Void {}))
        .into_widget()
    };
  }

  #[derive(Declare)]
  struct PipeParent;

  impl ComposeChild<'static> for PipeParent {
    type Child = BoxPipe<usize>;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      Void.into_widget()
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
