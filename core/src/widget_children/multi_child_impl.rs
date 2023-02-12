use super::*;

/// Trait specify what child a multi child widget can have, and the target type
/// after widget compose its child.
pub trait MultiWithChild<M, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

pub struct MultiChildWidget<W> {
  pub widget: W,
  pub children: Vec<Widget>,
}

impl<R, C, M> MultiWithChild<M, C> for R
where
  R: MultiChild,
  C: FillInChildren<M>,
{
  type Target = MultiChildWidget<R>;

  fn with_child(self, child: C) -> Self::Target {
    let mut children = vec![];
    child.fill_in(&mut children);
    MultiChildWidget { widget: self, children }
  }
}

impl<W, C, M> MultiWithChild<M, C> for MultiChildWidget<W>
where
  C: FillInChildren<M>,
{
  type Target = Self;
  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    child.fill_in(&mut self.children);
    self
  }
}

impl<R: Render + MultiChild + 'static> IntoWidget<NotSelf<()>> for MultiChildWidget<R> {
  fn into_widget(self) -> Widget {
    let MultiChildWidget { widget, children } = self;
    Widget::Render {
      render: Box::new(widget),
      children: Some(children),
    }
  }
}

trait FillInChildren<M> {
  fn fill_in(self, children: &mut Vec<Widget>);
}

impl FillInChildren<[(); 0]> for Widget {
  fn fill_in(self, children: &mut Vec<Widget>) { children.push(self) }
}

impl<C, M> FillInChildren<[M; 0]> for C
where
  C: IntoWidget<NotSelf<M>>,
{
  fn fill_in(self, children: &mut Vec<Widget>) { children.push(self.into_widget()) }
}

impl<D, M> FillInChildren<[M; 1]> for Stateful<DynWidget<D>>
where
  D: IntoIterator + 'static,
  D::Item: IntoWidget<M>,
  M: ImplMarker,
{
  fn fill_in(self, children: &mut Vec<Widget>) { children.push(DynRender::new(self).into_widget()) }
}

impl<C, M> FillInChildren<[M; 2]> for C
where
  C: IntoIterator,
  C::Item: IntoWidget<M>,
  M: ImplMarker,
{
  fn fill_in(self, children: &mut Vec<Widget>) {
    children.extend(self.into_iter().map(IntoWidget::into_widget))
  }
}
