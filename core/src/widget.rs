use crate::render_object::RenderObject;

use std::fmt::{Debug, Formatter, Result};

pub mod key;
mod row_layout;
pub mod text;

pub use key::{Key, KeyDetect};
pub use row_layout::Row;
pub use text::Text;

/// A widget represented by other widget compose.
pub trait CombinationWidget: Debug {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  /// Describes the part of the user interface represented by this widget.
  fn build<'a>(&self) -> Widget<'a>;
}

/// RenderWidget is a widget has its render object to display self.
pub trait RenderWidget: Debug {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn create_render_object(&self) -> Box<dyn RenderObject + Send + Sync>;
}

/// a widget has a child.
pub trait SingleChildWidget<'a> {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn split(self: Box<Self>) -> (Box<dyn RenderWidget + 'a>, Widget<'a>);
}

/// a widget has multi child
pub trait MultiChildWidget<'a> {
  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  fn split(self: Box<Self>) -> (Box<dyn RenderWidget + 'a>, Vec<Widget<'a>>);
}

pub enum Widget<'a> {
  Combination(Box<dyn CombinationWidget + 'a>),
  Render(Box<dyn RenderWidget + 'a>),
  SingleChild(Box<dyn SingleChildWidget<'a> + 'a>),
  MultiChild(Box<dyn MultiChildWidget<'a> + 'a>),
}
impl<'a> Debug for Widget<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Widget::Render(r) => r.fmt(f),
      Widget::Combination(c) => c.fmt(f),
      Widget::SingleChild(_) => f.write_str("SingleChild"),
      Widget::MultiChild(_) => f.write_str("MultiChild"),
    }
  }
}

impl<'a> Widget<'a> {
  pub fn key(&self) -> Option<&Key> {
    match self {
      Widget::Combination(w) => w.key(),
      Widget::Render(w) => w.key(),
      Widget::SingleChild(w) => w.key(),
      Widget::MultiChild(w) => w.key(),
    }
  }
}

pub trait CombinationWidgetInto<'a> {
  fn to_widget(self) -> Widget<'a>;
}

impl<'a, W: CombinationWidget + 'a> CombinationWidgetInto<'a> for W {
  fn to_widget(self) -> Widget<'a> { Widget::Combination(Box::new(self)) }
}

pub trait RenderWidgetInto<'a> {
  fn to_widget(self) -> Widget<'a>;
}

impl<'a, W: RenderWidget + 'a> RenderWidgetInto<'a> for W {
  fn to_widget(self) -> Widget<'a> { Widget::Render(Box::new(self)) }
}

pub trait SingleChildWidgetInto<'a> {
  fn to_widget(self) -> Widget<'a>;
}

impl<'a, W: SingleChildWidget<'a> + 'a> SingleChildWidgetInto<'a> for W {
  fn to_widget(self) -> Widget<'a> { Widget::SingleChild(Box::new(self)) }
}

pub trait MultiChildWidgetInto<'a> {
  fn to_widget(self) -> Widget<'a>;
}

impl<'a, W: MultiChildWidget<'a> + 'a> MultiChildWidgetInto<'a> for W {
  fn to_widget(self) -> Widget<'a> { Widget::MultiChild(Box::new(self)) }
}
