use crate::render::*;
use std::fmt::Debug;

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

  /// convert to [`Widget`](Widget), not override this method, will remove after
  /// Rust generic specialization finished
  fn to_widget<'a>(self) -> Widget<'a>
  where
    Self: Sized + 'a,
  {
    Widget::Combination(Box::new(self))
  }
}

/// a widget has a child.
pub trait SingleChildWidget: RenderWidgetSafety {
  /// called by framework to take child from this widget, and only called once.
  fn take_child<'a>(&mut self) -> Widget<'a>
  where
    Self: 'a;
}

/// a widget has multi child
pub trait MultiChildWidget: RenderWidgetSafety {
  /// called by framework to take children from this widget, and only called
  /// once.
  fn take_children<'a>(&mut self) -> Vec<Widget<'a>>
  where
    Self: 'a;
}

#[derive(Debug)]
pub enum Widget<'a> {
  Combination(Box<dyn CombinationWidget + 'a>),
  Render(Box<dyn RenderWidgetSafety + 'a>),
  SingleChild(Box<dyn SingleChildWidget + 'a>),
  MultiChild(Box<dyn MultiChildWidget + 'a>),
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

  pub fn same_type_widget(&self, other: &Widget) -> bool {
    match self {
      Widget::Combination(_) => matches!(other, Widget::Combination(_)),
      Widget::Render(_) => matches!(other, Widget::Render(_)),
      Widget::SingleChild(_) => matches!(other, Widget::SingleChild(_)),
      Widget::MultiChild(_) => matches!(other, Widget::MultiChild(_)),
    }
  }
}

pub trait IntoWidget {
  fn to_widget<'a>(self) -> Widget<'a>
  where
    Self: 'a;
}

impl<W: RenderWidgetSafety> IntoWidget for W {
  default fn to_widget<'a>(self) -> Widget<'a>
  where
    Self: 'a,
  {
    Widget::Render(Box::new(self))
  }
}

impl<W: MultiChildWidget> IntoWidget for W {
  fn to_widget<'a>(self) -> Widget<'a>
  where
    Self: 'a,
  {
    Widget::MultiChild(Box::new(self))
  }
}
