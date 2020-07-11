//! Framework use special widgets to listens to corresponding events, Three are
//! two raw listeners [`PointerListener`](pointers::PointerListener),
//! [`KeyboardListener`](keyboard::KeyboardListener). `Holiday` dispatch event
//! like web's bubble phase, always from the leaf to root.
use crate::widget::widget_tree::WidgetId;
pub(crate) mod dispatch;
pub mod pointers;
pub use pointers::{PointerEvent, PointerListener};
pub use winit::event::ModifiersState;

/// Event itself contains the properties and methods which are common to all
/// events
pub trait Event {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event::current_target when the event handler is called during the bubbling
  /// phase of the event.
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
pub struct EventCommon {
  pub target: WidgetId,
  pub current_target: WidgetId,
  pub composed_path: Vec<WidgetId>,
  pub modifiers: ModifiersState,
  pub cancel_bubble: std::cell::Cell<bool>,
}

impl<T: std::convert::AsRef<EventCommon>> Event for T {
  #[inline]
  fn target(&self) -> &WidgetId { &self.as_ref().target }
  #[inline]
  fn current_target(&self) -> &WidgetId { &self.as_ref().target }
  #[inline]
  fn composed_path(&self) -> &[WidgetId] { &self.as_ref().composed_path }
  #[inline]
  fn stop_bubbling(&self) { self.as_ref().cancel_bubble.set(false) }
  #[inline]
  fn modifiers(&self) -> ModifiersState { self.as_ref().modifiers }
}

pub(crate) fn add_listener<
  F: FnMut(&E) + 'static,
  E: std::convert::AsRef<EventCommon> + 'static,
>(
  holder: &mut Option<Box<dyn FnMut(&E)>>,
  mut handler: F,
) {
  *holder = if let Some(mut already) = holder.take() {
    Some(Box::new(move |event| {
      already(event);
      if !event.as_ref().cancel_bubble.get() {
        handler(event);
      }
    }))
  } else {
    Some(Box::new(handler))
  };
}

pub(crate) fn dispatch_event<E: std::convert::AsRef<EventCommon> + 'static>(
  holder: &mut Option<Box<dyn FnMut(&E)>>,
  event: &E,
) {
  if let Some(handler) = holder {
    handler(event);
  }
}
