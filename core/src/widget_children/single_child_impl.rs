use super::*;

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

impl<W, C, M> IntoWidget<NotSelf<M>> for WidgetPair<W, C>
where
  W: IntoSingleParent,
  C: IntoSingleChild<M>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget::Render {
      render: widget.into_single_parent(),
      children: child.into_single_child(),
    }
  }
}

trait IntoSingleParent {
  fn into_single_parent(self) -> Box<dyn Render>;
}

trait IntoSingleChild<M> {
  fn into_single_child(self) -> Option<Vec<Widget>>;
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

impl<W: IntoWidget<M>, M: ImplMarker> IntoSingleChild<[M; 0]> for W {
  fn into_single_child(self) -> Option<Vec<Widget>> { Some(vec![self.into_widget()]) }
}

impl<W: IntoWidget<M>, M: ImplMarker> IntoSingleChild<[M; 1]> for Option<W> {
  fn into_single_child(self) -> Option<Vec<Widget>> { self.map(|c| vec![c.into_widget()]) }
}
