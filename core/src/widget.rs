//! widget is a cheap config to detect how user interface should be display
use crate::render_object::RenderObject;
use ::herald::prelude::*;
use subject::LocalSubject;

mod key;
mod text;
pub use key::KeyDetect;
pub use text::Text;

/// `WidgetStates` can return a subscribable stream which emit a `()` value,
/// when widget state changed.
pub trait WidgetStates<'a> {
  fn changed_emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>>;
}

/// A widget represented by other widget compose.
pub trait CombinationWidget<'a>: WidgetStates<'a> {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String;
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
pub trait RenderWidget<'a>: WidgetStates<'a> {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String;

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
}

impl<'a, T> WidgetStates<'a> for T {
  #[inline]
  default fn changed_emitter(
    &mut self,
    _notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    None
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
