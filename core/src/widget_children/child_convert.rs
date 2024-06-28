use super::Pair;
use crate::{
  builtin_widgets::{FatObj, Void},
  context::BuildCtx,
  pipe::*,
  widget::*,
};

/// Trait for conversions type as a child of widget, it is similar to `Into` but
/// with a const marker to automatically implement all possible conversions
/// without implementing conflicts.  So you should not directly implement this
/// trait. Implement `Into` instead.
pub trait IntoChild<C, const M: usize> {
  fn into_child(self, ctx: &BuildCtx) -> C;
}

pub const INTO_CONVERT: usize = 0;

// `Into` conversion.
impl<T: Into<C>, C> IntoChild<C, INTO_CONVERT> for T {
  #[inline(always)]
  fn into_child(self, _: &BuildCtx) -> C { self.into() }
}

// All possible widget conversions.
macro_rules! impl_into_widget_child {
  ($($m:ident),*) => {
    $(
      // `IntoWidgetStrict` is utilized here to prevent conflicts with the `Into` trait bounds.
      impl<T: IntoWidgetStrict<$m>> IntoChild<Widget, $m> for T {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Widget { self.into_widget_strict(ctx) }
      }
    )*
  };
}

impl_into_widget_child!(COMPOSE, RENDER, COMPOSE_CHILD, FN);

macro_rules! impl_into_option_widget_child {
  ($($m: ident), *) => {
    $(
      // `IntoWidgetStrict` is utilized here to prevent conflicts with the `Into` trait bounds.
      impl<T: IntoWidgetStrict<$m>> IntoChild<Option<Widget>, $m> for Option<T> {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Option<Widget> {
          self.map(|v| v.into_widget_strict(ctx))
        }
      }

      // `IntoWidgetStrict` is utilized here to prevent conflicts with the `Into` trait bounds.
      impl<T: IntoWidgetStrict<$m>> IntoChild<Option<Widget>, $m> for T {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Option<Widget> {
          Some(self.into_widget_strict(ctx))
        }
      }
    )*
  };
}

impl_into_option_widget_child!(COMPOSE, RENDER, COMPOSE_CHILD, FN);

impl<V, S, F, const M: usize> IntoChild<Widget, M> for MapPipe<Option<V>, S, F>
where
  V: IntoWidget<M> + 'static,
  S: InnerPipe,
  S::Value: 'static,
  F: FnMut(S::Value) -> Option<V> + 'static,
{
  fn into_child(self, ctx: &BuildCtx) -> Widget { self.into_widget(ctx) }
}

impl<V, S, F, const M: usize> IntoChild<Widget, M> for FinalChain<Option<V>, S, F>
where
  V: IntoWidget<M> + 'static,
  S: InnerPipe<Value = Option<V>>,
  F: FnOnce(ValueStream<Option<V>>) -> ValueStream<Option<V>> + 'static,
{
  fn into_child(self, ctx: &BuildCtx) -> Widget { self.into_widget(ctx) }
}

impl<const M: usize, V: IntoWidget<M> + 'static> IntoChild<Widget, M>
  for Box<dyn Pipe<Value = Option<V>>>
{
  fn into_child(self, ctx: &BuildCtx) -> Widget { self.into_widget(ctx) }
}

impl<T: 'static> IntoChild<BoxPipe<T>, FN> for T {
  fn into_child(self, _: &BuildCtx) -> BoxPipe<T> { BoxPipe::value(self) }
}
/// Trait for conversions type as a compose child.
pub trait ChildFrom<V, M> {
  fn child_from(value: V, ctx: &BuildCtx) -> Self;
}

impl<T> ChildFrom<T, ()> for T {
  #[inline]
  fn child_from(value: T, _: &BuildCtx) -> Self { value }
}

impl<C, T: FromAnother<C, M>, M> ChildFrom<C, (M,)> for T {
  #[inline]
  #[track_caller]
  fn child_from(value: C, ctx: &BuildCtx) -> Self { FromAnother::from_another(value, ctx) }
}

/// Convert from another type to the compose child.
pub trait FromAnother<V, M> {
  fn from_another(value: V, _: &BuildCtx) -> Self;
}

// W -> Widget
crate::widget::multi_build_replace_impl! {
  impl<W: {#} > FromAnother<W, &dyn {#}> for Widget {
    #[inline]
    fn from_another(value: W, ctx: &BuildCtx) -> Self { value.build(ctx) }
  }
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<V: {#} + 'static, PP> FromAnother<PP, Box<dyn {#}>> for Widget
  where
    PP: InnerPipe<Value = Option<V>>,
  {
    #[inline]
    fn from_another(value: PP, ctx: &BuildCtx) -> Self {
      crate::pipe::pipe_option_to_widget!(value, ctx)
    }
  }
}

impl<M, T: 'static, V> FromAnother<T, [M; 0]> for BoxPipe<V>
where
  V: ChildFrom<T, M> + 'static,
{
  #[inline]
  fn from_another(value: T, ctx: &BuildCtx) -> Self {
    BoxPipe::value(ChildFrom::child_from(value, ctx))
  }
}

impl<T, V, M> FromAnother<T, [M; 1]> for BoxPipe<V>
where
  T: Pipe + 'static,
  V: ChildFrom<T::Value, M> + 'static,
{
  fn from_another(value: T, ctx: &BuildCtx) -> Self {
    let handle = ctx.handle();
    let pipe = value.map(move |v| {
      handle
        .with_ctx(|ctx| ChildFrom::child_from(v, ctx))
        .unwrap()
    });
    BoxPipe::pipe(Box::new(pipe))
  }
}

impl<T, Item, V, M> FromAnother<T, [M; 2]> for BoxPipe<Vec<V>>
where
  T: Pipe + 'static,
  T::Value: IntoIterator<Item = Item>,
  V: ChildFrom<Item, M> + 'static,
{
  fn from_another(value: T, ctx: &BuildCtx) -> Self {
    let handle = ctx.handle();
    let pipe = value.map(move |v| {
      handle
        .with_ctx(|ctx| {
          let v = v
            .into_iter()
            .map(|v| ChildFrom::child_from(v, ctx))
            .collect::<Vec<_>>();
          ChildFrom::child_from(v, ctx)
        })
        .unwrap()
    });
    BoxPipe::pipe(Box::new(pipe))
  }
}

impl<F> FromAnother<F, ()> for GenWidget
where
  F: FnMut(&BuildCtx) -> Widget + 'static,
{
  fn from_another(value: F, _: &BuildCtx) -> Self { Self::new(value) }
}

// W --- C ---> Option<C>
impl<W, C, M> FromAnother<W, [M; 0]> for Option<C>
where
  C: ChildFrom<W, M>,
{
  #[inline]
  fn from_another(value: W, ctx: &BuildCtx) -> Self { Some(ChildFrom::child_from(value, ctx)) }
}

// Option<W>  ---> Option<C>
impl<W, C, M> FromAnother<Option<C>, [M; 1]> for Option<W>
where
  W: FromAnother<C, M>,
{
  #[inline]
  fn from_another(value: Option<C>, ctx: &BuildCtx) -> Self {
    value.map(|v| FromAnother::from_another(v, ctx))
  }
}

// Pair<W, C> --> Pair<W2, C2>
impl<W, W2, C, C2, M1, M2> FromAnother<Pair<W2, C2>, [(M1, M2); 0]> for Pair<W, C>
where
  W: FromAnother<W2, M1>,
  C: FromAnother<C2, M2>,
{
  #[inline]
  fn from_another(value: Pair<W2, C2>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair { parent: W::from_another(widget, ctx), child: C::from_another(child, ctx) }
  }
}

impl<W, W2, C, M> FromAnother<Pair<W2, C>, [M; 1]> for Pair<W, C>
where
  W: FromAnother<W2, M>,
{
  #[inline]
  fn from_another(value: Pair<W2, C>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair { parent: W::child_from(widget, ctx), child }
  }
}

impl<W, C, C2, M> FromAnother<Pair<W, C2>, [M; 2]> for Pair<W, C>
where
  C: FromAnother<C2, M>,
{
  #[inline]
  fn from_another(value: Pair<W, C2>, ctx: &BuildCtx) -> Self {
    let Pair { parent: widget, child } = value;
    Pair { parent: widget, child: C::from_another(child, ctx) }
  }
}

impl<T1, T2, M> FromAnother<FatObj<T1>, [M; 0]> for FatObj<T2>
where
  T2: FromAnother<T1, M>,
{
  fn from_another(value: FatObj<T1>, ctx: &BuildCtx) -> Self {
    value.map(|v| FromAnother::from_another(v, ctx))
  }
}

impl<T1, T2, M> FromAnother<T1, [M; 1]> for FatObj<T2>
where
  T2: ChildFrom<T1, M>,
{
  #[inline]
  fn from_another(value: T1, ctx: &BuildCtx) -> Self {
    FatObj::new(ChildFrom::child_from(value, ctx))
  }
}
