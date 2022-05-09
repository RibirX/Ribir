use crate::{context::EventCtx, prelude::Context, widget::widget_tree::WidgetId};
use std::ptr::NonNull;

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
#[derive(Clone)]
pub struct EventCommon {
  pub(crate) target: WidgetId,
  pub(crate) current_target: WidgetId,
  pub(crate) cancel_bubble: bool,
  pub(crate) prevent_default: bool,
  context: NonNull<Context>,
}

impl EventCommon {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event::current_target when the event handler is called during the bubbling
  /// phase of the event.
  #[inline]
  pub fn target(&self) -> WidgetId { self.target }
  /// A reference to the currently registered target for the event. This is the
  /// object to which the event is currently slated to be sent. It's possible
  /// this has been changed along the way through retargeting.
  #[inline]
  pub fn current_target(&self) -> WidgetId { self.current_target }
  /// Prevent event bubbling to parent.
  #[inline]
  pub fn stop_bubbling(&mut self) { self.cancel_bubble = true }
  /// Return it the event is canceled to bubble to parent.
  #[inline]
  pub fn bubbling_canceled(&self) -> bool { self.cancel_bubble }
  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  #[inline]
  pub fn prevent_default(&mut self) { self.prevent_default = true; }
  /// Represents the current state of the keyboard modifiers
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.context().modifiers() }

  #[inline]
  pub fn context<'a>(&'a self) -> EventCtx<'a> {
    // Safety: framework promise event context only live in event dispatch and
    // there is no others to share `Context`.

    EventCtx::new(self.current_target(), unsafe { self.context.as_ref() })
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

impl EventCommon {
  pub(crate) fn new(target: WidgetId, context: &Context) -> Self {
    Self {
      target,
      current_target: target,
      cancel_bubble: <_>::default(),
      prevent_default: <_>::default(),
      context: NonNull::from(context),
    }
  }
}
