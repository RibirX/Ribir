use crate::render_object::RenderObject;

use std::fmt::{Debug, Formatter, Result};

pub mod key;
mod row_layout;
pub mod text;

pub use key::{Key, KeyDetect};
pub use row_layout::Row;
pub use text::Text;

/// A widget represented by other widget compose.
pub trait CombinationWidget<'a>: Debug {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> Widget;
}

/// RenderWidget is a widget has its render object to display self.
pub trait RenderWidget<'a>: Debug {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn create_render_object(&self) -> Box<dyn RenderObject>;
}

/// a widget has a child.
pub trait SingleChildWidget<'a> {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn split(self: Box<Self>) -> (Box<dyn for<'r> RenderWidget<'r>>, Widget);
}

/// a widget has multi child
pub trait MultiChildWidget<'a> {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn split(self: Box<Self>)
  -> (Box<dyn for<'r> RenderWidget<'r>>, Vec<Widget>);
}

pub enum Widget {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
  SingleChild(Box<dyn for<'a> SingleChildWidget<'a>>),
  MultiChild(Box<dyn for<'a> MultiChildWidget<'a>>),
}
impl Debug for Widget {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Widget::Render(r) => r.fmt(f),
      Widget::Combination(c) => c.fmt(f),
      Widget::SingleChild(_) => f.write_str("SingleChild"),
      Widget::MultiChild(_) => f.write_str("MultiChild"),
    }
  }
}

impl Widget {
  pub fn key(&self) -> Option<&Key> {
    match self {
      Widget::Combination(w) => w.key(),
      Widget::Render(w) => w.key(),
      Widget::SingleChild(w) => w.key(),
      Widget::MultiChild(w) => w.key(),
    }
  }
}

pub trait CombinationWidgetInto {
  fn to_widget(self) -> Widget;
}

impl<W: for<'r> CombinationWidget<'r> + 'static> CombinationWidgetInto for W {
  fn to_widget(self) -> Widget { Widget::Combination(Box::new(self)) }
}

pub trait RenderWidgetInto {
  fn to_widget(self) -> Widget;
}

impl<W: for<'r> RenderWidget<'r> + 'static> RenderWidgetInto for W {
  fn to_widget(self) -> Widget { Widget::Render(Box::new(self)) }
}

pub trait SingleChildWidgetInto {
  fn to_widget(self) -> Widget;
}

impl<W: for<'r> SingleChildWidget<'r> + 'static> SingleChildWidgetInto for W {
  fn to_widget(self) -> Widget { Widget::SingleChild(Box::new(self)) }
}

pub trait MultiChildWidgetInto {
  fn to_widget(self) -> Widget;
}

impl<W: for<'r> MultiChildWidget<'r> + 'static> MultiChildWidgetInto for W {
  fn to_widget(self) -> Widget { Widget::MultiChild(Box::new(self)) }
}
