pub use crate::prelude::*;

/// A marker trait to tell Ribir a widget can have one child.
pub trait SingleChildWidget: RenderWidget {
  fn have(self, child: BoxedWidget) -> SingleChild<Self>
  where
    Self: Sized,
  {
    SingleChild { widget: self, child }
  }
}

pub struct SingleChild<S> {
  widget: S,
  child: BoxedWidget,
}

pub type BoxedSingleChild = Box<SingleChild<Box<dyn RenderWidgetSafety>>>;

impl<S> SingleChild<S> {
  #[inline]
  pub fn unzip(self) -> (S, BoxedWidget) { (self.widget, self.child) }
}

impl<S: SingleChildWidget> SingleChild<S> {
  pub fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    let boxed = Box::new(SingleChild { widget, child: self.child });
    BoxedWidget::SingleChild(boxed)
  }
}

impl<S> std::ops::Deref for SingleChild<S> {
  type Target = S;
  #[inline]
  fn deref(&self) -> &S { &self.widget }
}

impl<R> std::ops::DerefMut for SingleChild<R> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

/// A marker trait to tell Ribir a widget can have multi child.
pub trait MultiChildWidget: RenderWidget + Sized {
  fn have(self, children: Vec<BoxedWidget>) -> MultiChild<Self> {
    MultiChild { widget: self, children }
  }

  #[inline]
  fn push(self, child: BoxedWidget) -> MultiChild<Self> {
    MultiChild { widget: self, children: vec![child] }
  }
}

pub struct MultiChild<M> {
  widget: M,
  children: Vec<BoxedWidget>,
}

pub type BoxedMultiChild = MultiChild<Box<dyn RenderWidgetSafety>>;

impl<M> MultiChild<M> {
  #[inline]
  pub fn unzip(self) -> (M, Vec<BoxedWidget>) { (self.widget, self.children) }
}

impl<M: MultiChildWidget> MultiChild<M> {
  pub fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    BoxedWidget::MultiChild(MultiChild { widget, children: self.children })
  }

  #[inline]
  pub fn push(mut self, child: BoxedWidget) -> Self {
    self.children.push(child);
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
