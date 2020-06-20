use crate::widget::widget_tree::WidgetId;
pub(crate) mod dispatch;
mod pointers;
pub use winit::event::ModifiersState;

/// Event itself contains the properties and methods which are common to all
/// events
#[derive(Debug)]
pub(crate) struct Event {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event.currentTarget when the event handler is called during the bubbling
  /// or capturing phase of the event.
  pub target: WidgetId,
  /// A reference to the currently registered target for the event. This is the
  /// object to which the event is currently slated to be sent. It's possible
  /// this has been changed along the way through retargeting.
  pub current_target: WidgetId,
  /// The composed_path of the Event interface returns the event’s path which is
  /// an array of the objects on which listeners will be invoked
  pub composed_path: Vec<WidgetId>,
  /// Represents the current state of the keyboard modifiers
  pub modifiers: ModifiersState,
  /// Prevent event bubbling to parent.
  pub(crate) cancel_bubble: std::cell::Cell<bool>,
}

pub macro common_event_method($field: ident) {
  #[inline]
  pub(crate) fn common(&mut self) -> &mut Event { &mut self.$field }
  #[inline]
  pub fn target(&self) -> &WidgetId { &self.$field.target }
  #[inline]
  pub fn current_target(&self) -> &WidgetId { &self.$field.target }
  #[inline]
  pub fn composed_path(&self) -> &[WidgetId] { &self.$field.composed_path }
  #[inline]
  pub fn stop_bubbling(&self) { self.$field.cancel_bubble.set(false) }
}
