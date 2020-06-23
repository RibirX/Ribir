use crate::widget::widget_tree::WidgetId;
pub(crate) mod dispatch;
mod pointers;
pub use winit::event::ModifiersState;

/// Event itself contains the properties and methods which are common to all
/// events
pub trait Event {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event.currentTarget when the event handler is called during the bubbling
  /// or capturing phase of the event.
  fn target(&self) -> &WidgetId;
  /// A reference to the currently registered target for the event. This is the
  /// object to which the event is currently slated to be sent. It's possible
  /// this has been changed along the way through retargeting.
  fn current_target(&self) -> &WidgetId;
  /// The composed_path of the Event interface returns the event’s path which is
  /// an array of the objects on which listeners will be invoked
  fn composed_path(&self) -> &[WidgetId];
  /// Prevent event bubbling to parent.
  fn stop_bubbling(&self);
  /// Represents the current state of the keyboard modifiers
  fn modifiers(&self) -> ModifiersState;
}

#[derive(Debug, Clone)]
pub(crate) struct EventCommon {
  pub target: WidgetId,
  pub current_target: WidgetId,
  pub composed_path: Vec<WidgetId>,
  pub modifiers: ModifiersState,
  pub(crate) cancel_bubble: std::cell::Cell<bool>,
}

pub(crate) macro impl_common_event($ty:ty, $field: ident) {
  impl Event for $ty {
    #[inline]
    fn target(&self) -> &WidgetId { &self.$field.target }
    #[inline]
    fn current_target(&self) -> &WidgetId { &self.$field.target }
    #[inline]
    fn composed_path(&self) -> &[WidgetId] { &self.$field.composed_path }
    #[inline]
    fn stop_bubbling(&self) { self.$field.cancel_bubble.set(false) }
    #[inline]
    fn modifiers(&self) -> ModifiersState { self.$field.modifiers }
  }

  impl std::convert::AsMut<EventCommon> for $ty {
    fn as_mut(&mut self) -> &mut EventCommon { &mut self.$field }
  }
}
