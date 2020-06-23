use crate::render::*;
use std::{
  any::{Any, TypeId},
  fmt::Debug,
};
pub mod build_ctx;
pub mod key;
pub mod layout;
mod stateful;
pub mod text;
pub mod widget_tree;
pub mod window;
pub use build_ctx::BuildCtx;
pub use key::{Key, KeyDetect};
pub use layout::row_col_layout::RowColumn;
pub use stateful::{Stateful, StatefulRef};
pub use text::Text;

/// The common behavior for widgets, also support to downcast to special widget.
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  /// Convert a stateless widget to stateful, use it get a cell ref to modify
  /// the widget.
  fn into_stateful(self, ctx: &BuildCtx) -> Stateful<Self>
  where
    Self: Sized,
  {
    Stateful::new(ctx.tree.clone(), self)
  }

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key>>(self, key: K) -> KeyDetect
  where
    Self: Sized,
  {
    KeyDetect::new(key, self)
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Debug {
  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> Box<dyn Widget>;
}

/// a widget has a child.
pub trait SingleChildWidget: RenderWidgetSafety {
  /// called by framework to take child from this widget, and only called once.
  fn take_child(&mut self) -> Box<dyn Widget>;
}

/// a widget has multi child
pub trait MultiChildWidget: RenderWidgetSafety {
  /// called by framework to take children from this widget, and only called
  /// once.
  fn take_children(&mut self) -> Vec<Box<dyn Widget>>;
}

pub enum WidgetClassify<'a> {
  Combination(&'a dyn CombinationWidget),
  Render(&'a dyn RenderWidgetSafety),
  SingleChild(&'a dyn SingleChildWidget),
  MultiChild(&'a dyn MultiChildWidget),
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
  SingleChild(&'a mut dyn SingleChildWidget),
  MultiChild(&'a mut dyn MultiChildWidget),
}

impl<'a> WidgetClassify<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassify::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassify::MultiChild(_)) }
}

impl<'a> WidgetClassifyMut<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassifyMut::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassifyMut::MultiChild(_)) }
}

/// We should also implement Widget for RenderWidgetSafety, SingleChildWidget
/// and MultiChildWidget, but can not do it before rust specialization finished.
/// So just CombinationWidget implemented it, this is user use most, and others
/// provide a macro to do it.
impl<'a, T: CombinationWidget + Any + 'a> Widget for T {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }
}

impl<T: CombinationWidget> !RenderWidget for T {}
impl<T: RenderWidget> !CombinationWidget for T {}
impl<T: MultiChildWidget> !SingleChildWidget for T {}
impl<T: SingleChildWidget> !MultiChildWidget for T {}

impl<W: Widget> From<W> for Box<dyn Widget> {
  #[inline]
  fn from(w: W) -> Self { Box::new(w) }
}

impl dyn Widget {
  pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
    if (&*self).type_id() == TypeId::of::<T>() {
      let ptr = self as *mut dyn Widget as *mut T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&mut *ptr) }
    } else {
      None
    }
  }

  pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn Widget as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      None
    }
  }
}

pub macro render_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }
}

pub macro single_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::SingleChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::SingleChild(self) }
}

pub macro multi_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::MultiChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::MultiChild(self) }
}
