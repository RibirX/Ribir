use crate::{context::BuildCtx, pipe::*, widget::*};

/// Trait for conversions type as a child of widget, it is similar to `Into` but
/// with a const marker to automatically implement all possible conversions
/// without implementing conflicts.  So you should not directly implement this
/// trait. Implement `Into` instead.
pub trait IntoChild<C, const M: usize> {
  fn into_child(self) -> C;
}

pub const SELF: usize = 0;

impl<C> IntoChild<C, SELF> for C {
  #[inline(always)]
  fn into_child(self) -> C { self }
}

impl<C> IntoChild<Option<C>, SELF> for C {
  #[inline(always)]
  fn into_child(self) -> Option<C> { Some(self) }
}

impl<F: FnMut(&mut BuildCtx) -> Widget<'static> + 'static> IntoChild<GenWidget, 0> for F {
  #[inline]
  fn into_child(self) -> GenWidget { GenWidget::new(self) }
}

// All possible widget conversions.
macro_rules! impl_into_widget_child {
  ($($m:ident),*) => {
    $(
      impl<'a, T: IntoWidgetStrict<'a, $m> + 'a> IntoChild<Widget<'a>, $m> for T {
        #[inline(always)]
        fn into_child(self) -> Widget<'a> { self.into_widget_strict() }
      }
    )*
  };
}

impl_into_widget_child!(COMPOSE, RENDER, FN);

macro_rules! impl_into_option_widget_child {
  ($($m: ident), *) => {
    $(
      impl<'a, T: IntoWidgetStrict<'a, $m> + 'a> IntoChild<Option<Widget<'a>>, $m> for Option<T> {
        #[inline(always)]
        fn into_child(self) -> Option<Widget<'a>> {
          self.map(|v| v.into_widget_strict())
        }
      }

      impl<'a, T: IntoWidgetStrict<'a, $m> + 'a> IntoChild<Option<Widget<'a>>, $m> for T {
        #[inline(always)]
        fn into_child(self) -> Option<Widget<'a>> {
          Some(self.into_widget_strict())
        }
      }
    )*
  };
}

impl_into_option_widget_child!(COMPOSE, RENDER, FN);

impl<V, S, F, const M: usize> IntoChild<Widget<'static>, M> for MapPipe<Option<V>, S, F>
where
  Self: IntoWidget<'static, M>,
{
  fn into_child(self) -> Widget<'static> { self.into_widget() }
}

impl<V, S, F, const M: usize> IntoChild<Widget<'static>, M> for FinalChain<Option<V>, S, F>
where
  Self: IntoWidget<'static, M>,
{
  fn into_child(self) -> Widget<'static> { self.into_widget() }
}

impl<V, const M: usize> IntoChild<Widget<'static>, M> for Box<dyn Pipe<Value = Option<V>>>
where
  Self: IntoWidget<'static, M>,
{
  fn into_child(self) -> Widget<'static> { self.into_widget() }
}

impl<P: Pipe> IntoChild<BoxPipe<P::Value>, 0> for P {
  fn into_child(self) -> BoxPipe<P::Value> { BoxPipe::pipe(Box::new(self)) }
}

impl<'w, F> IntoChild<FnWidget<'w>, FN> for F
where
  F: FnOnce(&mut BuildCtx) -> Widget<'w> + 'w,
{
  #[inline]
  fn into_child(self) -> FnWidget<'w> { Box::new(self) }
}
