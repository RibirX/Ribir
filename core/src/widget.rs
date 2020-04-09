use crate::render_object::RenderObject;
use ::herald::prelude::*;

use std::fmt::{Debug, Formatter, Result};
use subject::LocalSubject;

pub mod key;
mod row_layout;
pub mod text;

pub use key::{Key, KeyDetect};
pub use row_layout::Row;
pub use text::Text;

/// `WidgetStates` can return a subscribable stream which emit a `()` value,
/// when widget state changed.
pub trait WidgetStates<'a> {
  #[inline]
  fn changed_emitter(
    &mut self,
    _notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    None
  }

  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget<'a>: WidgetStates<'a> + Debug {
  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> Widget;

  /// Return a Some-value which contain a subscribable stream to notify rebuild.
  /// Return a None-value if this widget never occur rebuild.
  /// By default, every widget state change will trigger rebuild, you can
  /// override the default implement to decide which state change really need
  /// rebuild, or just return `None` because your widget need rebuild never.
  fn rebuild_emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    self.changed_emitter(notifier)
  }
}

/// RenderWidget is a widget has its render object to display self.
pub trait RenderWidget<'a>: WidgetStates<'a> + Debug {
  fn create_render_object(&self) -> Box<dyn RenderObject>;
}

/// a widget has a child.
pub trait SingleChildWidget<'a>: WidgetStates<'a> {
  fn split(self: Box<Self>) -> (Box<dyn for<'r> RenderWidget<'r>>, Widget);
}

/// a widget has multi child
pub trait MultiChildWidget<'a>: WidgetStates<'a> {
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

impl<'a> WidgetStates<'a> for Widget {
  fn changed_emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    match self {
      Widget::Combination(w) => w.changed_emitter(notifier),
      Widget::Render(w) => w.changed_emitter(notifier),
      Widget::SingleChild(w) => w.changed_emitter(notifier),
      Widget::MultiChild(w) => w.changed_emitter(notifier),
    }
  }

  fn key(&self) -> Option<&Key> {
    match self {
      Widget::Combination(w) => w.key(),
      Widget::Render(w) => w.key(),
      Widget::SingleChild(w) => w.key(),
      Widget::MultiChild(w) => w.key(),
    }
  }
}

impl<'a, T: Herald<'a> + 'a> WidgetStates<'a> for T {
  #[inline]
  default fn changed_emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    Some(self.batched_change_stream(notifier).map(|_v| ()).box_it())
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