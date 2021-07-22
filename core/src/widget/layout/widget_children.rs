pub use crate::prelude::*;

/// A marker trait to tell Ribir a widget can have one child.
pub trait SingleChildWidget: RenderWidget {
  // todo: a better name
  fn with_child(self, child: Box<dyn Widget>) -> SingleChild<Self>
  where
    Self: Sized,
  {
    SingleChild { widget: self, child }
  }
}

#[derive(Widget)]
pub struct SingleChild<S> {
  widget: S,
  child: Box<dyn Widget>,
}

pub type BoxedSingleChild = SingleChild<Box<dyn RenderWidgetSafety>>;

impl<S> SingleChild<S> {
  #[inline]
  pub fn unzip(self) -> (S, Box<dyn Widget>) { (self.widget, self.child) }
}

impl<S: SingleChildWidget> BoxWidget for SingleChild<S> {
  fn box_it(self) -> Box<dyn Widget> {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    SingleChild { widget, child: self.child }.box_it()
  }
}

impl<S: SingleChildWidget> std::ops::Deref for SingleChild<S> {
  type Target = S;
  #[inline]
  fn deref(&self) -> &S { &self.widget }
}

impl<R: SingleChildWidget> std::ops::DerefMut for SingleChild<R> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

/// A marker trait to tell Ribir a widget can have multi child.
pub trait MultiChildWidget: RenderWidget {
  // todo: a better name
  fn with_children(self, children: Vec<Box<dyn Widget>>) -> MultiChild<Self> {
    MultiChild { widget: self, children }
  }

  // todo: a better name
  fn from_iter<T: IntoIterator<Item = Box<dyn Widget>>>(self, iter: T) -> MultiChild<Self> {
    MultiChild {
      widget: self,
      children: iter.into_iter().collect(),
    }
  }
}

#[derive(Widget)]
pub struct MultiChild<M> {
  widget: M,
  children: Vec<Box<dyn Widget>>,
}

pub type BoxedMultiChild = MultiChild<Box<dyn RenderWidgetSafety>>;

impl<M> MultiChild<M> {
  #[inline]
  pub fn unzip(self) -> (M, Vec<Box<dyn Widget>>) { (self.widget, self.children) }
}

impl<M: MultiChildWidget> BoxWidget for MultiChild<M> {
  fn box_it(self) -> Box<dyn Widget> {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    MultiChild { widget, children: self.children }.box_it()
  }
}

impl<R: RenderWidget> std::ops::Deref for MultiChild<R> {
  type Target = R;
  #[inline]
  fn deref(&self) -> &R { &self.widget }
}

impl<R: RenderWidget> std::ops::DerefMut for MultiChild<R> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

impl<R: RenderWidget> MultiChild<R> {
  #[inline]
  pub fn push(mut self, child: Box<dyn Widget>) -> Self {
    self.children.push(child);
    self
  }
}
