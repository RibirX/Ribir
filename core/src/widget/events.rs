use crate::{
  render::{layout_store::LayoutStore, render_tree::RenderTree},
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::{cell::Cell, ptr::NonNull};

pub(crate) mod dispatcher;
mod pointers;
use crate::widget::window::RawWindow;
pub use pointers::*;
use std::{cell::RefCell, rc::Rc};
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
  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  fn prevent_default(&self);
  /// Represents the current state of the keyboard modifiers
  fn modifiers(&self) -> ModifiersState;

  fn widget_tree(&self) -> &WidgetTree;

  fn render_tree(&self) -> &RenderTree;
}

#[derive(Clone)]
pub struct EventCommon {
  pub target: WidgetId,
  pub current_target: WidgetId,
  pub modifiers: ModifiersState,
  pub cancel_bubble: Cell<bool>,
  pub prevent_default: Cell<bool>,
  pub window: Rc<RefCell<Box<dyn RawWindow>>>,
  widget_tree: NonNull<WidgetTree>,
  render_tree: NonNull<RenderTree>,
}

impl<T: std::convert::AsRef<EventCommon>> Event for T {
  #[inline]
  fn target(&self) -> WidgetId { self.as_ref().target }
  #[inline]
  fn current_target(&self) -> WidgetId { self.as_ref().current_target }
  #[inline]
  fn stop_bubbling(&self) { self.as_ref().cancel_bubble.set(true) }
  #[inline]
  fn prevent_default(&self) { self.as_ref().prevent_default.set(true) }
  #[inline]
  fn modifiers(&self) -> ModifiersState { self.as_ref().modifiers }
  #[inline]
  fn widget_tree(&self) -> &WidgetTree { unsafe { &*self.as_ref().widget_tree.as_ref() } }
  #[inline]
  fn render_tree(&self) -> &RenderTree { unsafe { &*self.as_ref().render_tree.as_ref() } }
}

impl std::fmt::Debug for EventCommon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.target)
      .field("current_target", &self.current_target)
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

impl EventCommon {
  pub fn new(
    modifiers: ModifiersState,
    target: WidgetId,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    widget_tree: NonNull<WidgetTree>,
    render_tree: NonNull<RenderTree>,
  ) -> Self {
    Self {
      modifiers,
      target,
      current_target: target,
      cancel_bubble: <_>::default(),
      prevent_default: <_>::default(),
      window,
      render_tree,
      widget_tree,
    }
  }

  pub fn layout_store(&self) -> &LayoutStore { todo!() }
}
