pub use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
#[marker]
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
#[marker]
pub trait MultiChild {}

pub trait SingleChildHave {
  #[inline]
  fn have_child<C, M>(self, child: C) -> SingleChildWidget<Self>
  where
    C: ChildConsumer<M, Target = SingleConsumer>,
    M: ?Sized,
    Self: Sized,
  {
    SingleChildWidget::new(self, child)
  }

  #[inline]
  fn have_expr_child(self, c: ExprWidget<SingleConsumer>) -> SingleChildWidget<Self>
  where
    Self: Sized,
  {
    SingleChildWidget::from_expr_child(self, c)
  }
}

pub trait MultiChildHave {
  #[inline]
  fn have_child<M, C>(self, child: C) -> MultiChildWidget<Self>
  where
    C: ChildConsumer<M>,
    M: ?Sized,
    Self: Sized,
  {
    MultiChildWidget::new(self, child)
  }

  #[inline]
  fn have_expr_child<R>(self, w: ExprWidget<R>) -> MultiChildWidget<Self>
  where
    ExprWidget<R>: IntoWidget<R>,
    Self: Sized,
  {
    self.have_child(w.into_widget())
  }
}
/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeSingleChild {
  fn compose_single_child(
    this: StateWidget<Self>,
    child: Option<Widget>,
    ctx: &mut BuildCtx,
  ) -> Widget
  where
    Self: Sized;
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its children.
pub trait ComposeMultiChild {
  fn compose_multi_child(
    this: StateWidget<Self>,
    children: Vec<Widget>,
    ctx: &mut BuildCtx,
  ) -> Widget
  where
    Self: Sized;
}

pub trait ChildConsumer<M: ?Sized> {
  type Target;
  fn consume(self, cb: &mut dyn FnMut(Widget)) -> Self::Target;
}

pub struct SingleConsumer;
pub struct MultiConsumer;

pub struct SingleChildWidget<W> {
  pub(crate) widget: W,
  pub(crate) child: Option<Widget>,
}

impl<M, W> ChildConsumer<dyn IntoWidget<M>> for W
where
  M: ?Sized,
  W: IntoWidget<M>,
{
  type Target = SingleConsumer;
  #[inline]
  fn consume(self, cb: &mut dyn FnMut(Widget)) -> SingleConsumer {
    cb(self.into_widget());
    SingleConsumer
  }
}

impl<W, M> ChildConsumer<Option<&dyn IntoWidget<M>>> for Option<W>
where
  M: ?Sized,
  W: IntoWidget<M>,
{
  type Target = SingleConsumer;
  #[inline]
  fn consume(self, cb: &mut dyn FnMut(Widget)) -> SingleConsumer {
    if let Some(w) = self {
      cb(w.into_widget())
    }
    SingleConsumer
  }
}

impl<W, M> ChildConsumer<dyn Iterator<Item = dyn IntoWidget<M>>> for W
where
  M: ?Sized,
  W: Iterator,
  W::Item: IntoWidget<M>,
{
  type Target = MultiConsumer;
  #[inline]
  fn consume(self, cb: &mut dyn FnMut(Widget)) -> MultiConsumer {
    self.for_each(|w| cb(w.into_widget()));
    MultiConsumer
  }
}

impl<S> SingleChildWidget<S> {
  #[inline]
  pub fn new<C, M>(widget: S, c: C) -> SingleChildWidget<S>
  where
    C: ChildConsumer<M, Target = SingleConsumer>,
    M: ?Sized,
  {
    let mut child = None;
    c.consume(&mut |w| child = Some(w));
    Self { widget, child }
  }

  #[inline]
  pub fn from_expr_child(widget: S, c: ExprWidget<SingleConsumer>) -> SingleChildWidget<S> {
    Self { widget, child: Some(c.into_widget()) }
  }

  #[inline]
  pub fn unzip(self) -> (S, Option<Widget>) { (self.widget, self.child) }
}

pub struct MultiChildWidget<W> {
  pub widget: W,
  pub children: Vec<Widget>,
}

impl<W> MultiChildWidget<W> {
  pub fn new<M: ?Sized, C>(widget: W, c: C) -> Self
  where
    C: ChildConsumer<M>,
  {
    let mut children = vec![];
    c.consume(&mut |w| children.push(w));
    Self { widget, children }
  }

  #[inline]
  pub fn unzip(self) -> (W, Vec<Widget>) { (self.widget, self.children) }
}

impl<W> MultiChildWidget<W> {
  #[inline]
  pub fn have_child<M, C>(mut self, child: C) -> MultiChildWidget<W>
  where
    C: ChildConsumer<M>,
    M: ?Sized,
    Self: Sized,
  {
    child.consume(&mut |w| self.children.push(w));
    self
  }

  #[inline]
  pub fn have_expr_child<R>(mut self, w: ExprWidget<R>) -> MultiChildWidget<W>
  where
    ExprWidget<R>: IntoWidget<R>,
    Self: Sized,
  {
    self.children.push(w.into_widget());
    self
  }
}

impl IntoWidget<dyn SingleChild> for SingleChildWidget<Box<dyn Render>> {
  #[inline]
  fn into_widget(self) -> Widget { Widget(WidgetInner::SingleChild(Box::new(self))) }
}

impl<W> IntoWidget<dyn Render> for SingleChildWidget<W>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let widget: Box<dyn Render> = Box::new(self.widget);
    let single_child = SingleChildWidget { widget, child: self.child };
    Widget(WidgetInner::SingleChild(Box::new(single_child)))
  }
}

impl<W> IntoWidget<dyn Render> for SingleChildWidget<Stateful<W>>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let single_child = SingleChildWidget {
      widget: self.widget.into_render_node(),
      child: self.child,
    };
    Widget(WidgetInner::SingleChild(Box::new(single_child)))
  }
}

impl<W> IntoWidget<dyn Compose> for SingleChildWidget<W>
where
  W: ComposeSingleChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeSingleChild::compose_single_child(self.widget.into(), self.child, ctx)
    })))
  }
}

impl<W> IntoWidget<dyn Compose> for SingleChildWidget<Stateful<W>>
where
  W: ComposeSingleChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeSingleChild::compose_single_child(StateWidget::Stateful(self.widget), self.child, ctx)
    })))
  }
}

impl IntoWidget<dyn MultiChild> for MultiChildWidget<Box<dyn Render>> {
  #[inline]
  fn into_widget(self) -> Widget { Widget(WidgetInner::MultiChild(self)) }
}

impl<W> IntoWidget<dyn Render> for MultiChildWidget<W>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let widget: Box<dyn Render> = Box::new(self.widget);
    let multi_child = MultiChildWidget { widget, children: self.children };
    Widget(WidgetInner::MultiChild(multi_child))
  }
}

impl<W> IntoWidget<dyn Render> for MultiChildWidget<Stateful<W>>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let widget: Box<dyn Render> = self.widget.into_render_node();
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
      ComposeMultiChild::compose_multi_child(self.widget.into(), self.children, ctx)
    })))
  }
}

impl<W> IntoWidget<dyn Compose> for MultiChildWidget<Stateful<W>>
where
  W: ComposeMultiChild + 'static,
{
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(move |ctx| {
      ComposeMultiChild::compose_multi_child(StateWidget::Stateful(self.widget), self.children, ctx)
    })))
  }
}

impl<T: SingleChild> SingleChildHave for T {}

impl<T: MultiChild> MultiChildHave for T {}

impl<T: ComposeSingleChild> SingleChild for T {}

impl<T: ComposeMultiChild> MultiChild for T {}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn fix_stateful_compose_single_have_child() {
    let _ = ScrollableWidget {
      scrollable: Scrollable::Both,
      pos: Point::zero(),
    }
    .into_stateful()
    .have_child(Void);
  }
}
