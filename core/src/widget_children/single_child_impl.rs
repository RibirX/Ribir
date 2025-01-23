use super::*;
use crate::pipe::InnerPipe;

/// This trait allows an `Option` of `SingleChild` to compose child.
pub trait OptionSingleChild {
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c>;
}

impl<P> OptionSingleChild for Option<P>
where
  P: SingleChild,
{
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
    let child = child.into_child_single();
    if let Some(parent) = self {
      parent.with_child(child)
    } else {
      child.expect("Either the parent or the child must exist.")
    }
  }
}

impl<'c, W, const M: usize> IntoChildSingle<'c, M> for W
where
  W: IntoWidget<'c, M>,
{
  fn into_child_single(self) -> Option<Widget<'c>> { Some(self.into_widget()) }
}

impl<'c, W, const M: usize> IntoChildSingle<'c, M> for Option<W>
where
  W: IntoWidget<'c, M>,
{
  fn into_child_single(self) -> Option<Widget<'c>> { self.map(IntoWidget::into_widget) }
}

impl<T: SingleChild> SingleChild for FatObj<T> {
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
    self
      .map(|parent| parent.with_child(child))
      .into_widget()
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> {
    let this = *self;
    if !this.has_class() {
      this.into_widget()
    } else {
      panic!("A FatObj should not have a class attribute when acting as a single parent")
    }
  }
}

macro_rules! impl_single_child_methods_for_pipe {
  () => {
    fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
      compose_single_child(self.into_parent_widget(), child.into_child_single())
    }

    fn into_parent(self: Box<Self>) -> Widget<'static> { self.into_parent_widget() }
  };
}

impl<S, V, F> SingleChild for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: SingleChild,
{
  impl_single_child_methods_for_pipe!();
}

impl<S, V, F> SingleChild for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: SingleChild,
{
  impl_single_child_methods_for_pipe!();
}

impl<V> SingleChild for Box<dyn Pipe<Value = V>>
where
  V: SingleChild,
{
  impl_single_child_methods_for_pipe!();
}

macro_rules! impl_single_child_methods_for_pipe_option {
  () => {
    fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
      let parent = self
        .map(|w| w.map_or_else(|| Void.into_widget(), V::into_widget))
        .into_parent_widget();
      compose_single_child(parent, child.into_child_single())
    }

    fn into_parent(self: Box<Self>) -> Widget<'static> {
      self
        .map(|w| w.map_or_else(|| Void.into_widget(), V::into_widget))
        .into_parent_widget()
    }
  };
}
impl<S, V, F> SingleChild for MapPipe<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: SingleChild,
{
  impl_single_child_methods_for_pipe_option!();
}

impl<S, V, F> SingleChild for FinalChain<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: SingleChild,
{
  impl_single_child_methods_for_pipe_option!();
}

impl<V> SingleChild for Box<dyn Pipe<Value = Option<V>>>
where
  V: SingleChild,
{
  impl_single_child_methods_for_pipe_option!();
}

impl<T> SingleChild for T
where
  T: StateReader<Value: SingleChild> + IntoWidget<'static, RENDER>,
{
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
    compose_single_child(self.into_widget(), child.into_child_single())
  }

  #[inline]
  fn into_parent(self: Box<Self>) -> Widget<'static> { self.into_widget() }
}

impl SingleChild for Box<dyn SingleChild> {
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
    compose_single_child(self.into_parent(), child.into_child_single())
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> { (*self).into_parent() }
}

pub fn compose_single_child<'c>(parent: Widget<'c>, child: Option<Widget<'c>>) -> Widget<'c> {
  if let Some(child) = child { Widget::new(parent, vec![child]) } else { parent }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::MockBox;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = |_: &BuildCtx| -> Widget {
      mock_box
        .clone()
        .with_child(mock_box.clone().with_child(mock_box))
        .into_widget()
    };
  }

  #[test]
  fn fix_mock_box_compose_pipe_option_widget() {
    fn _x(w: BoxPipe<Option<Widget<'static>>>) {
      MockBox { size: ZERO_SIZE }.with_child(w.into_pipe());
    }
  }
}
