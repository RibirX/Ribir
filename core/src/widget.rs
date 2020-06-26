use crate::render::*;
use std::{any::Any, fmt::Debug};

pub mod key;
pub mod layout;
pub mod text;
pub mod widget_tree;
pub mod window;
pub use key::{Key, KeyDetect};
pub use layout::row_col_layout::RowColumn;
pub use text::Text;

/// The common behavior for widgets, also support to downcast to special widget.
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  fn as_any(&self) -> &dyn Any;

  fn as_any_mut(&self) -> &dyn Any;
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

impl<'a> WidgetClassify<'a> {
  /// Return a Some-Value if this is a render widget, remember single child
  /// widget and multi child widget are render widget too. Otherwise return a
  /// None-Value.
  pub(crate) fn try_as_render(&self) -> Option<&dyn RenderWidgetSafety> {
    match self {
      WidgetClassify::Combination(_) => None,
      WidgetClassify::Render(w) => Some(w.as_render()),
      WidgetClassify::SingleChild(w) => Some(w.as_render()),
      WidgetClassify::MultiChild(w) => Some(w.as_render()),
    }
  }
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
  SingleChild(&'a mut dyn SingleChildWidget),
  MultiChild(&'a mut dyn MultiChildWidget),
}

impl<'a> WidgetClassifyMut<'a> {
  /// Return a Some-Value if this is a render widget, remember single child
  /// widget and multi child widget are render widget too. Otherwise return a
  /// None-Value.
  #[allow(dead_code)]
  pub(crate) fn try_as_render_mut(&mut self) -> Option<&mut dyn RenderWidgetSafety> {
    match self {
      WidgetClassifyMut::Combination(_) => None,
      WidgetClassifyMut::Render(w) => Some(w.as_render_mut()),
      WidgetClassifyMut::SingleChild(w) => Some(w.as_render_mut()),
      WidgetClassifyMut::MultiChild(w) => Some(w.as_render_mut()),
    }
  }
}

/// We should also implement Widget concrete methods for RenderWidgetSafety,
/// SingleChildWidget and MultiChildWidget, but can not do it before rust
/// specialization finished. So just CombinationWidget implemented it, this is
/// user use most, and others provide a macro to do it.
impl<'a, T: CombinationWidget + Any + 'a> Widget for T {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&self) -> &dyn Any { self }
}

impl<T: CombinationWidget> !RenderWidget for T {}
impl<T: RenderWidget> !CombinationWidget for T {}
impl<T: MultiChildWidget> !SingleChildWidget for T {}
impl<T: SingleChildWidget> !MultiChildWidget for T {}

impl<'a, W: Widget + 'a> From<W> for Box<dyn Widget + 'a> {
  #[inline]
  fn from(w: W) -> Self { Box::new(w) }
}

pub macro render_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&self) -> &dyn Any { self }
}

pub macro single_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::SingleChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::SingleChild(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&self) -> &dyn Any { self }
}

pub macro multi_child_widget_base_impl() {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::MultiChild(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::MultiChild(self) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&self) -> &dyn Any { self }
}
