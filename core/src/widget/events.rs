//! Framework use special widgets to listens to corresponding events, Three are
//! two raw listeners [`PointerListener`](pointers::PointerListener),
//! [`KeyboardListener`](keyboard::KeyboardListener). `Holiday` dispatch event
//! like web's bubble phase, always from the leaf to root.
use crate::widget::widget_tree::WidgetId;
use std::cell::Cell;
pub(crate) mod dispatcher;
pub mod pointers;
use crate::widget::window::RawWindow;
pub use pointers::*;
use std::{cell::RefCell, rc::Rc};
pub use winit::event::{ModifiersState, ScanCode, VirtualKeyCode};
pub mod focus;
pub use focus::*;

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

#[derive(Clone)]
pub struct EventCommon {
  pub target: WidgetId,
  pub current_target: WidgetId,
  pub composed_path: Vec<WidgetId>,
  pub modifiers: ModifiersState,
  pub cancel_bubble: Cell<bool>,
  pub window: Rc<RefCell<Box<dyn RawWindow>>>,
}

impl<T: std::convert::AsRef<EventCommon>> Event for T {
  #[inline]
  fn target(&self) -> &WidgetId { &self.as_ref().target }
  #[inline]
  fn current_target(&self) -> &WidgetId { &self.as_ref().target }
  #[inline]
  fn composed_path(&self) -> &[WidgetId] { &self.as_ref().composed_path }
  #[inline]
  fn stop_bubbling(&self) { self.as_ref().cancel_bubble.set(true) }
  #[inline]
  fn modifiers(&self) -> ModifiersState { self.as_ref().modifiers }
}

impl std::fmt::Debug for EventCommon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.target)
      .field("current_target", &self.current_target)
      .field("composed_path", &self.composed_path)
      .field("modifiers", &self.modifiers)
      .field("cancel_bubble", &self.cancel_bubble)
      .finish()
  }
}

impl std::convert::AsMut<EventCommon> for EventCommon {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { self }
}

impl std::convert::AsRef<EventCommon> for EventCommon {
  #[inline]
  fn as_ref(&self) -> &EventCommon { self }
}
