use super::{child_convert::IntoChild, *};
use crate::widget::ImplMarker;

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<M, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

/// A node of widget with not compose its child.
pub struct WidgetPair<W, C> {
  pub widget: W,
  pub child: C,
}

impl<W: SingleChild> SingleChild for Option<W> {}

impl<W: SingleChild, C> SingleWithChild<W, C> for W {
  type Target = WidgetPair<W, C>;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetPair { widget: self, child } }
}

impl<W, C1: SingleChild, C2> SingleWithChild<W, C2> for WidgetPair<W, C1> {
  type Target = WidgetPair<W, WidgetPair<C1, C2>>;

  fn with_child(self, c: C2) -> Self::Target {
    let WidgetPair { widget, child } = self;
    WidgetPair { widget, child: child.with_child(c) }
  }
}

impl<W, C, M> IntoWidget<NotSelf<M>> for WidgetPair<W, C>
where
  W: IntoSingleParent,
  C: IntoChild<M, Option<Widget>>,
  M: ImplMarker,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget::Render {
      render: widget.into_single_parent(),
      children: Some(child.into_child().map_or_else(Vec::default, |w| vec![w])),
    }
  }
}

impl<W, C, M1, M2> IntoWidget<NotSelf<[(M1, M2); 1]>> for WidgetPair<Option<W>, C>
where
  WidgetPair<W, C>: IntoWidget<M1>,
  C: IntoWidget<M2>,
  M1: ImplMarker,
  M2: ImplMarker,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      WidgetPair { widget, child }.into_widget()
    } else {
      child.into_widget()
    }
  }
}

impl<W, D, M1, M2> IntoWidget<NotSelf<[(M1, M2); 2]>>
  for WidgetPair<W, Stateful<DynWidget<Option<D>>>>
where
  WidgetPair<W, Widget>: IntoWidget<M1>,
  D: IntoChild<M2, Widget> + 'static,
  M1: ImplMarker,
  M2: ImplMarker,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_widget() }.into_widget()
  }
}

trait IntoSingleParent {
  fn into_single_parent(self) -> Box<dyn Render>;
}

impl<W: SingleChild + Render + 'static> IntoSingleParent for W {
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(self) }
}

impl<D> IntoSingleParent for Stateful<DynWidget<D>>
where
  D: SingleChild + Render + 'static,
{
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(DynRender::new(self)) }
}

impl<D> IntoSingleParent for Stateful<DynWidget<Option<D>>>
where
  D: SingleChild + Render + 'static,
{
  fn into_single_parent(self) -> Box<dyn Render> { Box::new(DynRender::new(self)) }
}

#[cfg(test)]
mod tests {
  use crate::test_helper::MockBox;

  use super::*;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = mock_box
      .clone()
      .with_child(mock_box.clone())
      .with_child(mock_box)
      .into_widget();
  }
}
