use crate::{
  context::{EventCtx, WidgetCtx},
  prelude::Context,
  widget::widget_tree::WidgetId,
};
use std::{cell::Cell, ptr::NonNull};

pub(crate) mod dispatcher;
mod pointers;
pub use pointers::*;
pub use winit::event::{ModifiersState, ScanCode, VirtualKeyCode};
mod focus;
pub use focus::*;
mod keyboard;
pub use keyboard::*;
mod character;
pub use character::*;
mod wheel;
pub use wheel::*;

/// Event itself contains the properties and methods which are common to all
/// events
pub trait Event {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event::current_target when the event handler is called during the bubbling
  /// phase of the event.
  fn target(&self) -> WidgetId;
  /// A reference to the currently registered target for the event. This is the
  /// object to which the event is currently slated to be sent. It's possible
  /// this has been changed along the way through retargeting.
  fn current_target(&self) -> WidgetId;
  /// Prevent event bubbling to parent.
  fn stop_bubbling(&self);

  /// Return it the event is canceled to bubble to parent.
  fn bubbling_canceled(&self) -> bool;

  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  fn prevent_default(&self);

  /// Represents the current state of the keyboard modifiers
  fn modifiers(&self) -> ModifiersState;

  fn context<'a>(&'a self) -> EventCtx<'a>;
}

#[derive(Clone)]
pub struct EventCommon {
  pub(crate) target: WidgetId,
  pub(crate) current_target: WidgetId,
  pub(crate) cancel_bubble: Cell<bool>,
  pub(crate) prevent_default: Cell<bool>,
  context: NonNull<Context>,
}

impl<T: std::convert::AsRef<EventCommon>> Event for T {
  #[inline]
  fn target(&self) -> WidgetId { self.as_ref().target }
  #[inline]
  fn current_target(&self) -> WidgetId { self.as_ref().current_target }
  #[inline]
  fn stop_bubbling(&self) { self.as_ref().cancel_bubble.set(true) }
  #[inline]
  fn bubbling_canceled(&self) -> bool { self.as_ref().cancel_bubble.get() }
  #[inline]
  fn prevent_default(&self) { self.as_ref().prevent_default.set(true) }
  #[inline]
  fn modifiers(&self) -> ModifiersState { self.context().context().modifiers }

  #[inline]
  fn context<'a>(&'a self) -> EventCtx<'a> {
    // Safety: framework promise event context only live in event dispatch and
    // there is no others to share `Context`.

    EventCtx::new(self.current_target(), unsafe {
      self.as_ref().context.as_ref()
    })
  }
}

impl std::fmt::Debug for EventCommon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.target)
      .field("current_target", &self.current_target)
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

impl EventCommon {
  pub fn new(target: WidgetId, context: &Context) -> Self {
    Self {
      target,
      current_target: target,
      cancel_bubble: <_>::default(),
      prevent_default: <_>::default(),
      context: NonNull::from(context),
    }
  }
}
