use crate::{context::BuildCtx, pipe::*, widget::*};

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
