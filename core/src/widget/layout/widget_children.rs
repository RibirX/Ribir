pub use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {
  #[inline]
  fn have_child<C, M: ?Sized>(self, child: C) -> SingleChildWidget<Self, C>
  where
    Self: Sized,
    SingleChildWidget<Self, C>: IntoWidget<M>,
  {
    SingleChildWidget::new(self, child)
  }
}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {
  #[inline]
  fn have_child<M: ?Sized, C: MultiChildMarker<M>>(self, child: C) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    let mut multi = MultiChildWidget { widget: self, children: vec![] };
    child.fill(&mut multi);
    multi
  }
}

pub trait Child<M: ?Sized> {}

pub trait MultiChildMarker<M: ?Sized> {
  fn fill<W>(self, multi: &mut MultiChildWidget<W>);
}
/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeSingleChild {
  #[inline]
  fn have_child<C, M: ?Sized>(self, child: C) -> SingleChildWidget<Self, C>
  where
    Self: Sized,
    SingleChildWidget<Self, C>: IntoWidget<M>,
  {
    SingleChildWidget::new(self, child)
  }

  fn compose_single_child(this: StateWidget<Self>, child: Widget, ctx: &mut BuildCtx) -> Widget
  where
    Self: Sized;
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its children.
pub trait ComposeMultiChild {
  #[inline]
  fn have_child<M: ?Sized, C: MultiChildMarker<M>>(self, child: C) -> MultiChildWidget<Self>
  where
    Self: Sized,
  {
    let mut multi = MultiChildWidget { widget: self, children: vec![] };
    child.fill(&mut multi);
    multi
  }

  fn compose_multi_child(
    this: StateWidget<Self>,
    children: Vec<Widget>,
    ctx: &mut BuildCtx,
  ) -> Widget
  where
    Self: Sized;
}

pub struct SingleChildWidget<W, C> {
  pub(crate) widget: W,
  pub(crate) child: C,
}

impl<W, C> SingleChildWidget<W, C> {
  #[inline]
  pub fn new(widget: W, child: C) -> Self { Self { widget, child } }

  #[inline]
  pub fn unzip(self) -> (W, C) { (self.widget, self.child) }
}

pub struct MultiChildWidget<W> {
  pub widget: W,
  pub children: Vec<Widget>,
}

impl<W> MultiChildWidget<W> {
  #[inline]
  pub fn unzip(self) -> (W, Vec<Widget>) { (self.widget, self.children) }
}

impl<W> MultiChildWidget<W> {
  #[inline]
  pub fn have_child<M: ?Sized, C: MultiChildMarker<M>>(mut self, child: C) -> Self {
    child.fill(&mut self);
    self
  }
}

impl<W> IntoWidget<(&dyn Render, Widget)> for SingleChildWidget<W, Widget>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Render(Box::new(widget));
    let children = Children::Single(Box::new(child));
    Widget { node: Some(node), children }
  }
}

impl<W> IntoWidget<(&dyn Compose, Widget)> for SingleChildWidget<W, Widget>
where
  W: ComposeSingleChild + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Compose(Box::new(move |ctx| {
      ComposeSingleChild::compose_single_child(widget.into(), child, ctx)
    }));
    let children = Children::None;
    Widget { node: Some(node), children }
  }
}

impl<W, M: ?Sized> IntoWidget<(&M, Widget)> for SingleChildWidget<W, ConstExprWidget<Widget>>
where
  SingleChildWidget<W, Widget>: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.expr }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Render)> for SingleChildWidget<W, C>
where
  W: SingleChild + Render + 'static,
  C: Render + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Compose, dyn Render)> for SingleChildWidget<W, C>
where
  W: ComposeSingleChild + 'static,
  C: Render + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Compose)> for SingleChildWidget<W, C>
where
  W: Render + SingleChild + 'static,
  C: Compose + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Compose, dyn Compose)> for SingleChildWidget<W, C>
where
  W: ComposeSingleChild + 'static,
  C: Compose + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, Option<&M2>)>
  for SingleChildWidget<W, ConstExprWidget<Option<C>>>
where
  W: IntoWidget<M1>,
  SingleChildWidget<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child.expr {
      SingleChildWidget { widget, child }.into_widget()
    } else {
      widget.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(Option<&M1>, &M2)>
  for SingleChildWidget<ConstExprWidget<Option<W>>, C>
where
  SingleChildWidget<W, C>: IntoWidget<M1>,
  C: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(widget) = widget.expr {
      SingleChildWidget { widget, child }.into_widget()
    } else {
      child.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized, M3: ?Sized> IntoWidget<(&M3, Option<&M1>, Option<&M2>)>
  for SingleChildWidget<ConstExprWidget<Option<W>>, ConstExprWidget<Option<C>>>
where
  W: IntoWidget<M1>,
  C: IntoWidget<M2>,
  SingleChildWidget<W, C>: IntoWidget<M3>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    match (widget.expr, child.expr) {
      (None, None) => Void.into_widget(),
      (None, Some(child)) => child.into_widget(),
      (Some(widget), None) => widget.into_widget(),
      (Some(widget), Some(child)) => SingleChildWidget { widget, child }.into_widget(),
    }
  }
}

impl<W, E, R, M: ?Sized> IntoWidget<(SingleResult<R>, &M)> for SingleChildWidget<ExprWidget<E>, W>
where
  E: FnMut() -> SingleResult<R> + 'static,
  R: SingleChild + Render + 'static,
  W: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let mut widget = widget.into_widget();
    widget.children = Children::Single(Box::new(child.into_widget()));

    widget
  }
}

impl<W, E, R, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, SingleResult<&M2>)>
  for SingleChildWidget<W, ExprWidget<E>>
where
  SingleChildWidget<W, Widget>: IntoWidget<M1>,
  E: FnMut() -> SingleResult<R> + 'static,
  R: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, &M2)>
  for SingleChildWidget<W, MultiChildWidget<C>>
where
  SingleChildWidget<W, Widget>: IntoWidget<M1>,
  MultiChildWidget<C>: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W1, W2, C, M2: ?Sized> IntoWidget<(&dyn Render, &M2)>
  for SingleChildWidget<W1, SingleChildWidget<W2, C>>
where
  W1: Render + SingleChild + 'static,
  SingleChildWidget<W2, C>: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W1, W2, C, M2: ?Sized> IntoWidget<(&dyn Compose, &M2)>
  for SingleChildWidget<W1, SingleChildWidget<W2, C>>
where
  W1: ComposeSingleChild + 'static,
  SingleChildWidget<W2, C>: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W> IntoWidget<dyn Render> for MultiChildWidget<W>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let MultiChildWidget { widget, children } = self;
    let node = WidgetNode::Render(Box::new(widget));
    let children = Children::Multi(children);
    Widget { node: Some(node), children }
  }
}

impl<W> IntoWidget<dyn Compose> for MultiChildWidget<W>
where
  W: ComposeMultiChild + 'static,
{
  fn into_widget(self) -> Widget {
    let MultiChildWidget { widget, children } = self;
    let node = WidgetNode::Compose(Box::new(move |ctx| {
      ComposeMultiChild::compose_multi_child(widget.into(), children, ctx)
    }));
    Widget {
      node: Some(node),
      children: Children::None,
    }
  }
}

impl<T: IntoWidget<M>, M: ?Sized> MultiChildMarker<M> for T {
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) { multi.children.push(self.into_widget()) }
}

impl<T, M: ?Sized> MultiChildMarker<dyn Iterator<Item = &M>> for ConstExprWidget<T>
where
  T: IntoIterator,
  T::Item: IntoWidget<M>,
{
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) {
    self
      .expr
      .into_iter()
      .for_each(|c| multi.children.push(c.into_widget()))
  }
}

impl<M: ?Sized, E, R> MultiChildMarker<ExprWidget<&M>> for ExprWidget<E>
where
  E: FnMut() -> SingleResult<R> + 'static,
  R: IntoWidget<M>,
{
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) { multi.children.push(self.into_widget()) }
}

impl<E> MultiChildMarker<Widget> for ExprWidget<E>
where
  E: FnMut() -> MultiResult + 'static,
{
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) {
    let w = self.into_multi_child();
    multi.children.push(w)
  }
}
