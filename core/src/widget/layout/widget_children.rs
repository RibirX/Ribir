use crate::dynamic_widget::ExprWidget;
pub use crate::prelude::*;

/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {
  #[inline]
  fn have_child(self, child: Widget) -> SingleChildWidget<Self>
  where
    Self: Sized,
  {
    SingleChildWidget { widget: self, child }
  }

  #[inline]
  fn have_expr_child<M: ?Sized, F: IntoWidgetSingleChild<M>>(
    self,
    child: F,
  ) -> SingleChildWidget<Self>
  where
    Self: Sized,
  {
    SingleChildWidget {
      widget: self,
      child: child.into_widget_single_child(),
    }
  }
}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {
  #[inline]
  fn have_child(self, child: Widget) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: vec![child.into_widget()],
    }
  }

  #[inline]
  fn have_expr_child<M: ?Sized, F: IntoWidget<M>>(self, child: Widget) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: vec![child.into_widget()],
    }
  }
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeSingleChild {
  fn compose_single_child(this: Stateful<Self>, child: Widget, ctx: &mut BuildCtx) -> Widget
  where
    Self: Sized;

  #[inline]
  fn have_child(self, child: Widget) -> SingleChildWidget<Self>
  where
    Self: Sized,
  {
    SingleChildWidget { widget: self, child }
  }

  #[inline]
  fn have_expr_child<M: ?Sized, F: IntoWidgetSingleChild<M>>(
    self,
    child: F,
  ) -> SingleChildWidget<Self>
  where
    Self: Sized,
  {
    SingleChildWidget {
      widget: self,
      child: child.into_widget_single_child(),
    }
  }
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its children.
pub trait ComposeMultiChild {
  fn compose_multi_child(this: Stateful<Self>, children: Vec<Widget>, ctx: &mut BuildCtx) -> Widget
  where
    Self: Sized;

  #[inline]
  fn have_child(self, child: Widget) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: vec![child.into_widget()],
    }
  }

  #[inline]
  fn have_expr_child<M: ?Sized, F: IntoWidget<M>>(self, child: Widget) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: vec![child.into_widget()],
    }
  }
}

/// trait use to limit expr at most one widget generate.
pub trait IntoWidgetSingleChild<M: ?Sized> {
  fn into_widget_single_child(self) -> Widget;
}

pub struct SingleChildWidget<W> {
  pub(crate) widget: W,
  pub(crate) child: Widget,
}

impl<S> SingleChildWidget<S> {
  #[inline]
  pub fn unzip(self) -> (S, Widget) { (self.widget, self.child) }

  #[inline]
  pub fn new(widget: S, child: Widget) -> SingleChildWidget<S> {
    SingleChildWidget { widget, child }
  }

  #[inline]
  pub fn from_expr_child<M: ?Sized, F: IntoWidgetSingleChild<M>>(
    widget: S,
    child: F,
  ) -> SingleChildWidget<S>
  where
    Self: Sized,
  {
    SingleChildWidget {
      widget,
      child: child.into_widget_single_child(),
    }
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
  pub fn have_child(mut self, widget: Widget) -> Self {
    self.children.push(widget);
    self
  }

  #[inline]
  pub fn have_expr_child<M: ?Sized, F: IntoWidget<M>>(self, child: Widget) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    MultiChildWidget {
      widget: self,
      children: vec![child.into_widget()],
    }
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

impl<F, R, M: ?Sized> IntoWidgetSingleChild<dyn IntoWidget<M>> for ExprWidget<F>
where
  F: FnMut() -> R + 'static,
  R: IntoWidget<M> + 'static,
  M: 'static,
{
  fn into_widget_single_child(self) -> Widget {
    let ExprWidget { mut expr, upstream } = self;
    ExprWidget {
      expr: move || std::iter::once(expr()),
      upstream,
    }
    .into_widget()
  }
}

impl<F, R, M: ?Sized> IntoWidgetSingleChild<dyn FnMut() -> M> for ExprWidget<F>
where
  F: FnMut() -> Option<R> + 'static,
  R: IntoWidget<M> + 'static,
  M: 'static,
{
  fn into_widget_single_child(self) -> Widget {
    let ExprWidget { mut expr, upstream } = self;
    ExprWidget {
      expr: move || expr().into_iter(),
      upstream,
    }
    .into_widget()
  }
}
