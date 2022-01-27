pub use crate::prelude::*;

/// A marker trait to tell Ribir a widget can have one child.
pub trait SingleChildWidget
where
  Self: IntoRender + Sized,
{
  fn have(self, child: BoxedWidget) -> SingleChild<Self> { SingleChild { widget: self, child } }
}

pub struct SingleChild<S> {
  pub(crate) widget: S,
  pub(crate) child: BoxedWidget,
}

impl<S> SingleChild<S> {
  #[inline]
  pub fn unzip(self) -> (S, BoxedWidget) { (self.widget, self.child) }
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
  Self: IntoRender + Sized,
{
  fn have_multi(self, children: Vec<BoxedWidget>) -> MultiChild<Self> {
    MultiChild { widget: self, children }
  }

  #[inline]
  fn have(self, c: BoxedWidget) -> MultiChild<Self> {
    MultiChild { widget: self, children: vec![c] }
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
  pub fn have(mut self, c: BoxedWidget) -> Self {
    self.children.push(c);
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

// Implementation for adding child to widget. If `specialization` finished,
// maybe we can use a more elegant way.
pub trait SingleComposeNormal<M> {
  type R;
  fn compose(self) -> Self::R
  where
    Self: Sized;
}

impl<W, C, M> SingleComposeNormal<M> for (W, C)
where
  W: SingleChildWidget,
  C: BoxWidget<M>,
{
  type R = SingleChild<W>;
  #[inline]
  fn compose(self) -> Self::R { self.0.have(self.1.box_it()) }
}

pub trait SingleComposeOption<M> {
  fn compose(self) -> BoxedWidget;
}

impl<W, M, C> SingleComposeOption<M> for (W, Option<C>)
where
  W: SingleChildWidget + 'static,
  C: BoxWidget<M> + 'static,
{
  #[inline]
  fn compose(self) -> BoxedWidget {
    let (p, c) = self;
    match c {
      Some(c) => (p, c).compose().box_it(),
      None => p.box_it(),
    }
  }
}

impl<W, M, C> SingleComposeOption<M> for (Option<W>, C)
where
  W: SingleChildWidget + 'static,
  C: BoxWidget<M>,
{
  #[inline]
  fn compose(self) -> BoxedWidget {
    let (p, c) = self;
    match p {
      Some(p) => (p, c).compose().box_it(),
      None => c.box_it(),
    }
  }
}

pub trait MultiComposeNormal<W, M> {
  fn compose(self) -> W;
}

impl<W, C, M> MultiComposeNormal<MultiChild<W>, M> for (MultiChild<W>, C)
where
  C: BoxWidget<M>,
  W: MultiChildWidget,
{
  #[inline]
  fn compose(self) -> MultiChild<W> { self.0.have(self.1.box_it()) }
}

impl<W, C, M> MultiComposeNormal<MultiChild<W>, M> for (W, C)
where
  C: BoxWidget<M>,
  W: MultiChildWidget,
{
  #[inline]
  fn compose(self) -> MultiChild<W> { self.0.have(self.1.box_it()) }
}

pub trait MultiComposeIter<W, M> {
  fn compose(self) -> W;
}

impl<C, W, M> MultiComposeIter<MultiChild<W>, M> for (MultiChild<W>, C)
where
  C: IntoIterator,
  C::Item: BoxWidget<M>,
{
  #[inline]
  fn compose(self) -> MultiChild<W> {
    let (mut p, c) = self;
    p.children.extend(c.into_iter().map(BoxWidget::box_it));
    p
  }
}

impl<C, W, M> MultiComposeIter<MultiChild<W>, M> for (W, C)
where
  C: IntoIterator,
  C::Item: BoxWidget<M>,
  W: MultiChildWidget,
{
  #[inline]
  fn compose(self) -> MultiChild<W> {
    let (widget, c) = self;
    MultiChild {
      widget,
      children: c.into_iter().map(BoxWidget::box_it).collect(),
    }
  }
}
