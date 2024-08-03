use super::*;
use crate::pipe::InnerPipe;

// We preserve the parent type, while the child must always be a widget. This is
// important to maintain the integrity of the template, as it may rely on
// accessing the parent type.
impl<'w, 'c: 'w, P, C, const M: usize> WithChild<'w, C, 0, M> for P
where
  P: Parent + 'static,
  C: IntoWidget<'c, M> + 'w,
{
  type Target = WidgetOf<'w, Self>;
  fn with_child(self, child: C) -> Self::Target {
    Pair { parent: self, child: child.into_widget() }
  }
}

// with option child
impl<'w, 'c: 'w, P, C, const M: usize> WithChild<'w, Option<C>, 0, M> for P
where
  P: SingleIntoParent,
  C: IntoWidget<'c, M> + 'w,
{
  type Target = Widget<'w>;
  fn with_child(self, child: Option<C>) -> Self::Target {
    if let Some(child) = child {
      Pair { parent: self, child: child.into_widget() }.into_widget()
    } else {
      self.into_parent()
    }
  }
}

// option parent with child
impl<'w, 'c: 'w, P, C, const M: usize> WithChild<'w, C, 0, M> for Option<P>
where
  P: SingleIntoParent,
  C: IntoWidget<'c, M> + 'w,
{
  type Target = Widget<'w>;

  fn with_child(self, child: C) -> Self::Target {
    if let Some(parent) = self {
      Pair { parent, child: child.into_widget() }.into_widget()
    } else {
      child.into_widget()
    }
  }
}

impl<'w, W: SingleIntoParent> IntoWidgetStrict<'w, RENDER> for WidgetOf<'w, W> {
  fn into_widget_strict(self) -> Widget<'w> {
    let f = move |ctx: &mut BuildCtx| {
      let Pair { parent, child } = self;

      parent
        .into_parent()
        .directly_compose_children(vec![child], ctx)
    };

    f.into_widget()
  }
}

/// This trait indicates that a type can serve as a parent widget for another
/// widget. It is similar to the `SingleChild` trait but includes the concept of
/// a pipe for `SingleChild`. The reason for this distinction is that the logic
/// for being a parent widget with a pipe differs from that of a regular render
/// widget. Therefore, the pipe does not implement the `SingleChild` trait but
/// instead utilizes the `Parent` trait to distinguish between the two.

trait Parent {}

impl<T: SingleChild> Parent for T {}
impl<T: Parent> Parent for FatObj<T> {}

impl<S, V: Parent, F> Parent for MapPipe<V, S, F> {}

impl<S, V: Parent, F> Parent for FinalChain<V, S, F> {}

impl<V: Parent> Parent for Box<dyn Pipe<Value = V>> {}

impl<S, V: Parent, F> Parent for MapPipe<Option<V>, S, F> {}

impl<S, V: Parent, F> Parent for FinalChain<Option<V>, S, F> {}

impl<V: Parent> Parent for Box<dyn Pipe<Value = Option<V>>> {}

/// This trait converts a parent into a widget. We require this trait because
/// the logic for being a parent widget with a pipe differs from that of a
/// regular render widget.
pub trait SingleIntoParent: 'static {
  fn into_parent(self) -> Widget<'static>;
}

// Implementation `IntoParent`
impl<P: SingleChild + IntoWidget<'static, RENDER>> SingleIntoParent for P {
  #[inline]
  fn into_parent(self) -> Widget<'static> { self.into_widget() }
}

impl<P: SingleIntoParent> SingleIntoParent for FatObj<P> {
  fn into_parent(self) -> Widget<'static> { self.map(|p| p.into_parent()).into_widget() }
}

impl<S, V, F> SingleIntoParent for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<S, V, F> SingleIntoParent for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<V> SingleIntoParent for Box<dyn Pipe<Value = V>>
where
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<S, V, F> SingleIntoParent for MapPipe<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { option_pipe_into_parent(self) }
}

impl<S, V, F> SingleIntoParent for FinalChain<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { option_pipe_into_parent(self) }
}

impl<V> SingleIntoParent for Box<dyn Pipe<Value = Option<V>>>
where
  V: SingleIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { option_pipe_into_parent(self) }
}

fn option_pipe_into_parent<const M: usize>(
  p: impl InnerPipe<Value = Option<impl IntoWidget<'static, M>>>,
) -> Widget<'static> {
  p.map(|w| move |_: &mut BuildCtx| w.map_or_else(|| Void.into_widget(), IntoWidget::into_widget))
    .into_parent_widget()
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
