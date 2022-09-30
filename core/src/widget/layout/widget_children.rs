pub use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeSingleChild {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget
  where
    Self: Sized;
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its children.
pub trait ComposeMultiChild {
  fn compose_multi_child(this: StateWidget<Self>, children: Vec<Widget>) -> Widget
  where
    Self: Sized;
}

pub trait HaveChild<M: ?Sized, C> {
  type Target;
  fn have_child(self, child: C) -> Self::Target;
}

pub struct SingleChildWidget<W, C> {
  pub(crate) widget: W,
  pub(crate) child: C,
}

pub struct MultiChildWidget<W> {
  pub widget: W,
  pub children: Vec<Widget>,
}

trait AllowSingle<M: ?Sized> {}
trait AllowMulti<M: ?Sized> {}

impl<W: MultiChild> AllowMulti<dyn MultiChild> for W {}
impl<W: ComposeMultiChild> AllowMulti<dyn ComposeMultiChild> for W {}

impl<W: SingleChild> AllowSingle<dyn SingleChild> for W {}
impl<W: ComposeSingleChild> AllowSingle<dyn ComposeSingleChild> for W {}

// Begin: implementation of `HaveChild` limit valid parent compose child.

impl<W: AllowSingle<M>, C, M: ?Sized> HaveChild<(&dyn SingleChild, &M), C> for W {
  type Target = SingleChildWidget<W, C>;
  #[inline]
  fn have_child(self, child: C) -> Self::Target { SingleChildWidget { widget: self, child } }
}

impl<W1, W2: HaveChild<M, C>, C, M: ?Sized> HaveChild<&M, C> for SingleChildWidget<W1, W2> {
  type Target = SingleChildWidget<W1, W2::Target>;

  fn have_child(self, child: C) -> Self::Target {
    let SingleChildWidget { widget, child: w2 } = self;
    SingleChildWidget { widget, child: w2.have_child(child) }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> HaveChild<(&dyn MultiChild, &M1, &M2), C> for W
where
  W: AllowMulti<M1>,
  C: FillMulti<M2>,
{
  type Target = MultiChildWidget<W>;
  #[inline]
  fn have_child(self, child: C) -> Self::Target {
    let mut multi = MultiChildWidget { widget: self, children: vec![] };
    child.fill(&mut multi);
    multi
  }
}

impl<W, C: FillMulti<M>, M: ?Sized> HaveChild<&M, C> for MultiChildWidget<W> {
  type Target = MultiChildWidget<W>;
  #[inline]
  fn have_child(mut self, child: C) -> Self::Target {
    child.fill(&mut self);
    self
  }
}

// todo: option expr (or const) widget, need limit
// multi support have multi / single
// option single only support have single or leaf.
impl<M: ?Sized, W: HaveChild<M, C>, C> HaveChild<Option<&M>, C> for Option<W> {
  type Target = SingleChildWidget<Option<W>, C>;

  #[inline]
  fn have_child(self, child: C) -> Self::Target { SingleChildWidget { widget: self, child } }
}

impl<M: ?Sized, C, W> HaveChild<ExprWidget<&M>, C> for ConstExprWidget<W>
where
  W: HaveChild<M, C>,
{
  type Target = W::Target;

  #[inline]
  fn have_child(self, child: C) -> Self::Target { self.expr.have_child(child) }
}

// implementation of IntoWidget

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

impl<W, C, M: ?Sized> IntoWidget<(&dyn Compose, &M)> for SingleChildWidget<W, C>
where
  W: ComposeSingleChild + 'static,
  C: IntoWidget<M> + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Compose(Box::new(move |_| {
      ComposeSingleChild::compose_single_child(widget.into(), child.into_widget())
    }));
    let children = Children::None;
    Widget { node: Some(node), children }
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

impl<W, C, M: ?Sized> IntoWidget<(&M, ConstExprWidget<C>)>
  for SingleChildWidget<W, ConstExprWidget<C>>
where
  SingleChildWidget<W, C>: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.expr }.into_widget()
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, Option<&M2>)>
  for SingleChildWidget<W, Option<C>>
where
  W: IntoWidget<M1>,
  SingleChildWidget<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child {
      SingleChildWidget { widget, child }.into_widget()
    } else {
      widget.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(Option<&M1>, &M2)>
  for SingleChildWidget<Option<W>, C>
where
  SingleChildWidget<W, C>: IntoWidget<M1>,
  C: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      SingleChildWidget { widget, child }.into_widget()
    } else {
      child.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized, M3: ?Sized> IntoWidget<(&M3, Option<&M1>, Option<&M2>)>
  for SingleChildWidget<Option<W>, Option<C>>
where
  W: IntoWidget<M1>,
  C: IntoWidget<M2>,
  SingleChildWidget<W, C>: IntoWidget<M3>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    match (widget, child) {
      (None, None) => Void.into_widget(),
      (None, Some(child)) => child.into_widget(),
      (Some(widget), None) => widget.into_widget(),
      (Some(widget), Some(child)) => SingleChildWidget { widget, child }.into_widget(),
    }
  }
}

impl<W, E, R, M: ?Sized> IntoWidget<(&dyn SingleChild, &M)> for SingleChildWidget<ExprWidget<E>, W>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
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

impl<W, E, R, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, ExprWidget<&M2>)>
  for SingleChildWidget<W, ExprWidget<E>>
where
  SingleChildWidget<W, Widget>: IntoWidget<M1>,
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: SingleDyn<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    SingleChildWidget { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C, M: ?Sized> IntoWidget<(&dyn Render, &M)> for SingleChildWidget<W, MultiChildWidget<C>>
where
  W: Render + SingleChild + 'static,
  MultiChildWidget<C>: IntoWidget<M>,
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
    let node = WidgetNode::Compose(Box::new(move |_| {
      ComposeMultiChild::compose_multi_child(widget.into(), children)
    }));
    Widget {
      node: Some(node),
      children: Children::None,
    }
  }
}

trait FillMulti<M: ?Sized> {
  fn fill<W>(self, multi: &mut MultiChildWidget<W>);
}

impl<T: IntoWidget<M>, M: ?Sized> FillMulti<&M> for T {
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) { multi.children.push(self.into_widget()) }
}

impl<T, M: ?Sized> FillMulti<ConstExprWidget<&M>> for ConstExprWidget<T>
where
  T: FillMulti<M>,
{
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) { self.expr.fill(multi) }
}

impl<T, M: ?Sized> FillMulti<dyn Iterator<Item = &M>> for ConstExprWidget<T>
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

impl<M: ?Sized, E, R> FillMulti<ExprWidget<&M>> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: IntoDynWidget<M>,
{
  #[inline]
  fn fill<W>(self, multi: &mut MultiChildWidget<W>) { multi.children.push(self.into_child()) }
}

// todo: impl have children for it and strip the exprWidget.
impl<R: SingleChild, E> SingleChild for ExprWidget<E> where E: FnMut(&mut BuildCtx) -> R {}
impl<R: MultiChild, E> MultiChild for ExprWidget<E> where E: FnMut(&mut BuildCtx) -> R {}
