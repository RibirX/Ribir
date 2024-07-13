use crate::{context::BuildCtx, pipe::*, widget::*};

/// Trait for conversions type as a child of widget, it is similar to `Into` but
/// with a const marker to automatically implement all possible conversions
/// without implementing conflicts.  So you should not directly implement this
/// trait. Implement `Into` instead.
pub trait IntoChild<C, const M: usize> {
  fn into_child(self, ctx: &BuildCtx) -> C;
}

pub const SELF: usize = 0;

impl<C> IntoChild<C, SELF> for C {
  #[inline(always)]
  fn into_child(self, _: &BuildCtx) -> C { self }
}

impl<C> IntoChild<Option<C>, SELF> for C {
  #[inline(always)]
  fn into_child(self, _: &BuildCtx) -> Option<C> { Some(self) }
}

impl<F: FnMut(&BuildCtx) -> Widget + 'static> IntoChild<GenWidget, 0> for F {
  #[inline]
  fn into_child(self, _: &BuildCtx) -> GenWidget { GenWidget::new(self) }
}

// All possible widget conversions.
macro_rules! impl_into_widget_child {
  ($($m:ident),*) => {
    $(
      // `IntoWidgetStrict` is utilized here to prevent conflicts with the `Self`
      impl<T: IntoWidgetStrict<$m>> IntoChild<Widget, $m> for T {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Widget { self.into_widget_strict(ctx) }
      }
    )*
  };
}

impl_into_widget_child!(COMPOSE, RENDER, FN);

macro_rules! impl_into_option_widget_child {
  ($($m: ident), *) => {
    $(
      impl<T: IntoWidgetStrict<$m>> IntoChild<Option<Widget>, $m> for Option<T> {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Option<Widget> {
          self.map(|v| v.into_widget_strict(ctx))
        }
      }

      impl<T: IntoWidgetStrict<$m>> IntoChild<Option<Widget>, $m> for T {
        #[inline(always)]
        fn into_child(self, ctx: &BuildCtx) -> Option<Widget> {
          Some(self.into_widget_strict(ctx))
        }
      }
    )*
  };
}

impl_into_option_widget_child!(COMPOSE, RENDER, FN);

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

impl<P: Pipe + 'static> IntoChild<BoxPipe<P::Value>, 0> for P {
  fn into_child(self, _: &BuildCtx) -> BoxPipe<P::Value> { BoxPipe::pipe(Box::new(self)) }
}
