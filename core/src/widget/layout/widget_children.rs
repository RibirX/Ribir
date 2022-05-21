use crate::dynamic_widget::ExprWidget;
pub use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {
  #[inline]
  fn have_child<C, M>(self, child: C) -> SingleChildWidget<Self>
  where
    C: IntoSingleChild<M>,
    M: ?Sized,
    Self: Sized,
  {
    SingleChildWidget::new(self, child.into_single_child())
  }
}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {
  #[inline]
  fn have_child<M, C>(self, child: C) -> MultiChildWidget<Self>
  where
    C: IntoMultiChild<M>,
    M: ?Sized,
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: child.into_multi_child().collect(),
    }
  }
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeSingleChild {
  fn compose_single_child(
    this: Stateful<Self>,
    child: Option<Widget>,
    ctx: &mut BuildCtx,
  ) -> Widget
  where
    Self: Sized;

  #[inline]
  fn have_child<C, M>(self, child: C) -> SingleChildWidget<Self>
  where
    C: IntoSingleChild<M>,
    M: ?Sized,
    Self: Sized,
  {
    SingleChildWidget::new(self, child.into_single_child())
  }
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its children.
pub trait ComposeMultiChild {
  fn compose_multi_child(this: Stateful<Self>, children: Vec<Widget>, ctx: &mut BuildCtx) -> Widget
  where
    Self: Sized;

  #[inline]
  fn have_child<M, C>(self, child: C) -> MultiChildWidget<Self>
  where
    C: IntoMultiChild<M>,
    M: ?Sized,
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: child.into_multi_child().collect(),
    }
  }
}
pub struct SingleChildWidget<W> {
  pub(crate) widget: W,
  pub(crate) child: Option<Widget>,
}

impl<S> SingleChildWidget<S> {
  #[inline]
  pub fn unzip(self) -> (S, Option<Widget>) { (self.widget, self.child) }

  #[inline]
  pub fn new(widget: S, child: Option<Widget>) -> SingleChildWidget<S> {
    SingleChildWidget { widget, child }
  }
}

pub struct MultiChildWidget<W> {
  pub widget: W,
  pub children: Vec<Widget>,
}

impl<M> MultiChildWidget<M> {
  #[inline]
  pub fn unzip(self) -> (M, Vec<Widget>) { (self.widget, self.children) }
}

impl<W> MultiChildWidget<W> {
  #[inline]
  pub fn have_child<M, C>(mut self, child: C) -> MultiChildWidget<W>
  where
    C: IntoMultiChild<M>,
    M: ?Sized,
    Self: Sized,
  {
    self.children.extend(child.into_multi_child());
    self
  }
}

impl<W> IntoWidget<dyn Render> for SingleChildWidget<W>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget);
    let single_child = SingleChildWidget { widget, child: self.child };
    Widget(WidgetInner::SingleChild(Box::new(single_child)))
  }
}

impl<W> IntoWidget<dyn Compose> for SingleChildWidget<W>
where
  W: ComposeSingleChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeSingleChild::compose_single_child(self.widget.into_stateful(), self.child, ctx)
    })))
  }
}

impl<W> IntoWidget<dyn Compose> for SingleChildWidget<Stateful<W>>
where
  W: ComposeSingleChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeSingleChild::compose_single_child(self.widget, self.child, ctx)
    })))
  }
}

pub trait IntoSingleChild<M: ?Sized> {
  fn into_single_child(self) -> Option<Widget>;
}

pub trait IntoMultiChild<M: ?Sized> {
  type IntoIter: Iterator<Item = Widget>;
  fn into_multi_child(self) -> Self::IntoIter;
}

impl IntoSingleChild<Widget> for Widget {
  #[inline]
  fn into_single_child(self) -> Option<Widget> { Some(self) }
}

impl<W: IntoWidget<M>, M: ?Sized> IntoSingleChild<(Self, M)> for ExprWidget<W> {
  #[inline]
  fn into_single_child(self) -> Option<Widget> { self.expr.into_widget().into() }
}

impl<W: IntoWidget<M>, M: ?Sized> IntoSingleChild<Option<&M>> for ExprWidget<Option<W>> {
  #[inline]
  fn into_single_child(self) -> Option<Widget> { self.expr.map(IntoWidget::into_widget) }
}

impl<F, R, M> IntoSingleChild<dyn FnMut() -> M> for ExprWidget<F>
where
  F: FnMut() -> R + 'static,
  R: IntoWidget<M> + 'static,
  M: ?Sized + 'static,
{
  #[inline]
  fn into_single_child(self) -> Option<Widget> { self.into_widget().into() }
}

impl<F, R, M> IntoSingleChild<(&dyn FnMut(), Option<&M>)> for ExprWidget<F>
where
  F: FnMut() -> Option<R> + 'static,
  R: IntoWidget<M> + 'static,
  M: ?Sized + 'static,
{
  fn into_single_child(self) -> Option<Widget> { self.into_widget().into() }
}

impl IntoMultiChild<Widget> for Widget {
  type IntoIter = std::iter::Once<Widget>;
  #[inline]
  fn into_multi_child(self) -> Self::IntoIter { std::iter::once(self) }
}

impl<W: IntoWidget<M>, M: ?Sized> IntoMultiChild<(Self, M)> for ExprWidget<W> {
  type IntoIter = std::iter::Once<Widget>;
  #[inline]
  fn into_multi_child(self) -> Self::IntoIter { std::iter::once(self.expr.into_widget()) }
}

impl<M, W> IntoMultiChild<dyn Iterator<Item = &M>> for ExprWidget<W>
where
  M: ?Sized,
  W: IntoIterator,
  W::Item: IntoWidget<M>,
{
  type IntoIter = std::iter::Map<W::IntoIter, fn(W::Item) -> Widget>;
  #[inline]
  fn into_multi_child(self) -> Self::IntoIter { self.expr.into_iter().map(IntoWidget::into_widget) }
}

impl<M, W> IntoMultiChild<ExprWidget<&M>> for ExprWidget<W>
where
  M: ?Sized,
  Self: IntoWidget<M>,
{
  type IntoIter = std::iter::Once<Widget>;
  #[inline]
  fn into_multi_child(self) -> Self::IntoIter { std::iter::once(self.into_widget()) }
}

impl<W> IntoWidget<dyn Render> for MultiChildWidget<W>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget);
    let multi_child = MultiChildWidget { widget, children: self.children };
    Widget(WidgetInner::MultiChild(multi_child))
  }
}

impl<W> IntoWidget<dyn Compose> for MultiChildWidget<W>
where
  W: ComposeMultiChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeMultiChild::compose_multi_child(self.widget.into_stateful(), self.children, ctx)
    })))
  }
}

impl<W> IntoWidget<dyn Compose> for MultiChildWidget<Stateful<W>>
where
  W: ComposeMultiChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeMultiChild::compose_multi_child(self.widget, self.children, ctx)
    })))
  }
}
