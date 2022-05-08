pub use crate::prelude::*;

/// A marker trait to tell Ribir a widget can have one child.
pub trait SingleChildWidget
where
  Self: Sized,
{
  #[inline]
  fn have_child<C: IntoOptionChild<M> + 'static, M>(self, child: C) -> SingleChild<Self> {
    SingleChild {
      widget: self,
      child: child.into_child(),
    }
  }
}

pub struct SingleChild<S> {
  pub(crate) widget: S,
  pub(crate) child: Option<BoxedWidget>,
}

impl<S> SingleChild<S> {
  #[inline]
  pub fn unzip(self) -> (S, Option<BoxedWidget>) { (self.widget, self.child) }
}

impl<S> std::ops::Deref for SingleChild<S> {
  type Target = S;
  #[inline]
  fn deref(&self) -> &S { &self.widget }
}

impl<S> std::ops::DerefMut for SingleChild<S> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

/// A marker trait to tell Ribir a widget can have multi child.
pub trait MultiChildWidget
where
  Self: Sized,
{
  #[inline]
  fn have_child<'a, C: IntoChildIterator<'a, M>, M: ?Sized>(self, child: C) -> MultiChild<Self> {
    MultiChild {
      widget: self,
      children: child.into_child_iter().collect(),
    }
  }
}

pub struct MultiChild<M> {
  pub(crate) widget: M,
  pub(crate) children: Vec<BoxedWidget>,
}

impl<M> MultiChild<M> {
  #[inline]
  pub fn unzip(self) -> (M, Vec<BoxedWidget>) { (self.widget, self.children) }
}

impl<M: MultiChildWidget> MultiChild<M> {
  #[inline]
  pub fn have_child<'a, C: IntoChildIterator<'a, Marker>, Marker: ?Sized>(
    mut self,
    child: C,
  ) -> Self {
    self.children.extend(child.into_child_iter());
    self
  }
}

impl<R> std::ops::Deref for MultiChild<R> {
  type Target = R;
  #[inline]
  fn deref(&self) -> &R { &self.widget }
}

impl<R> std::ops::DerefMut for MultiChild<R> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

impl<S: IntoStateful> IntoStateful for SingleChild<S> {
  type S = SingleChild<S::S>;
  #[inline]
  fn into_stateful(self) -> Self::S {
    SingleChild {
      widget: self.widget.into_stateful(),
      child: self.child,
    }
  }
}

impl<M: IntoStateful> IntoStateful for MultiChild<M> {
  type S = MultiChild<M::S>;
  #[inline]
  fn into_stateful(self) -> Self::S {
    MultiChild {
      widget: self.widget.into_stateful(),
      children: self.children,
    }
  }
}

pub trait IntoOptionChild<M> {
  fn into_child(self) -> Option<BoxedWidget>;
}

impl<W: BoxWidget<M>, M> IntoOptionChild<M> for W {
  #[inline]
  fn into_child(self) -> Option<BoxedWidget> { Some(self.box_it()) }
}

impl<W: BoxWidget<M>, M> IntoOptionChild<Option<M>> for Option<W> {
  #[inline]
  fn into_child(self) -> Option<BoxedWidget> { self.map(BoxWidget::box_it) }
}

pub trait IntoChildIterator<'a, M: ?Sized> {
  fn into_child_iter(self) -> Box<dyn Iterator<Item = BoxedWidget> + 'a>;
}

impl<'a, T: BoxWidget<M>, M> IntoChildIterator<'a, M> for T {
  fn into_child_iter(self) -> Box<dyn Iterator<Item = BoxedWidget>> {
    Box::new(Some(self.box_it()).into_iter())
  }
}

impl<'a, T, M> IntoChildIterator<'a, [M]> for T
where
  T: IntoIterator + 'a,
  T::Item: BoxWidget<M> + 'a,
  M: 'a,
{
  fn into_child_iter(self) -> Box<dyn Iterator<Item = BoxedWidget> + 'a> {
    Box::new(self.into_iter().map(BoxWidget::box_it))
  }
}

pub trait OptionHaveChild {
  type Target;
  fn have_child<C: BoxWidget<M> + 'static, M>(self, child: C) -> BoxedWidget;
}

impl<T> OptionHaveChild for Option<T>
where
  T: SingleChildWidget,
  SingleChild<T>: BoxWidget<RenderMarker>,
{
  type Target = T;

  fn have_child<C: BoxWidget<M> + 'static, M>(self, child: C) -> BoxedWidget {
    match self {
      Some(w) => w.have_child(child).box_it(),
      None => child.box_it(),
    }
  }
}
