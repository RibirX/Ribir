use std::{
  any::Any,
  ops::{Deref, DerefMut},
  ptr::NonNull,
};

use self::dispatcher::DispatchInfo;
use crate::{
  builtin_widgets::MixFlags,
  context::{WidgetCtx, WidgetCtxImpl},
  prelude::ProviderCtx,
  query::QueryHandle,
  widget_tree::{WidgetId, WidgetTree},
};

pub(crate) mod dispatcher;
pub use dispatcher::GrabPointer;
pub mod custom_event;
pub use custom_event::*;
mod pointers;
pub use pointers::*;
use ribir_geom::Point;
mod keyboard;
pub use keyboard::*;
mod character;
pub use character::*;
mod wheel;
use smallvec::SmallVec;
pub use wheel::*;
mod ime_pre_edit;
pub use ime_pre_edit::*;
mod lifecycle;
pub use lifecycle::*;

mod device_id;
pub use device_id::*;
pub(crate) mod focus_mgr;
pub use focus_mgr::*;
mod listener_impl_helper;

pub struct CommonEvent {
  pub(crate) id: WidgetId,
  // The framework guarantees the validity of this pointer at all times; therefore, refrain from
  // using a reference to prevent introducing lifetimes in `CommonEvent` and maintain cleaner code.
  tree: NonNull<WidgetTree>,
  provider_ctx: ProviderCtx,
  target: WidgetId,
  propagation: bool,
  prevent_default: bool,
}

impl CommonEvent {
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
  pub fn current_target(&self) -> WidgetId { self.id }
  /// Prevent event bubbling to parent.
  #[inline]
  pub fn stop_propagation(&mut self) { self.propagation = false }
  /// Whether the event is bubbling or not.
  #[inline]
  pub fn is_propagation(&self) -> bool { self.propagation }
  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  #[inline]
  pub fn prevent_default(&mut self) { self.prevent_default = true; }

  /// Whether the event is prevented the default action or not.
  #[inline]
  pub(crate) fn is_prevent_default(&self) -> bool { self.prevent_default }

  /// Represents the current state of the keyboard modifiers
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.pick_info(DispatchInfo::modifiers) }

  /// Returns `true` if the shift key is pressed.
  pub fn with_shift_key(&self) -> bool { self.modifiers().shift_key() }

  /// Returns `true` if the alt key is pressed.
  pub fn with_alt_key(&self) -> bool { self.modifiers().alt_key() }

  /// Returns `true` if the ctrl key is pressed.
  pub fn with_ctrl_key(&self) -> bool { self.modifiers().control_key() }
  /// Returns `true` if the logo key is pressed.
  pub fn with_logo_key(&self) -> bool { self.modifiers().super_key() }

  /// Returns true if the main modifier key in the
  /// current platform is pressed. Specifically:
  /// - the `logo` or command key (⌘) on macOS
  /// - the `control` key on other platforms
  pub fn with_command_key(&self) -> bool {
    #[cfg(target_os = "macos")]
    return self.with_logo_key();

    #[cfg(not(target_os = "macos"))]
    return self.with_ctrl_key();
  }

  /// The X, Y coordinate of the mouse pointer in global (window) coordinates.
  #[inline]
  pub fn global_pos(&self) -> Point { self.pick_info(DispatchInfo::global_pos) }

  /// The X, Y coordinate of the pointer in current target widget.
  #[inline]
  pub fn position(&self) -> Point { self.map_from_global(self.global_pos()) }

  /// The buttons being depressed (if any) in current state.
  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.pick_info(DispatchInfo::mouse_buttons) }

  /// The button number that was pressed (if applicable) when the mouse event
  /// was fired.
  #[inline]
  pub fn button_num(&self) -> u32 { self.mouse_buttons().bits().count_ones() }
}

pub enum Event {
  /// Fired when a widget is mounted to the tree.
  ///
  /// Occurs exactly once per widget lifetime.
  Mounted(LifecycleEvent),
  /// Event fired when the widget is performed layout. This event may fire
  /// multiple times in same frame if a widget modified after performed layout.
  PerformedLayout(LifecycleEvent),
  /// Fired when a widget is permanently removed from the tree.
  ///
  /// Occurs exactly once per widget lifetime.
  Disposed(LifecycleEvent),
  PointerDown(PointerEvent),
  PointerDownCapture(PointerEvent),
  PointerUp(PointerEvent),
  PointerUpCapture(PointerEvent),
  PointerMove(PointerEvent),
  PointerMoveCapture(PointerEvent),
  PointerCancelCapture(PointerEvent),
  PointerCancel(PointerEvent),
  PointerEnter(PointerEvent),
  PointerLeave(PointerEvent),
  Tap(PointerEvent),
  TapCapture(PointerEvent),
  ImePreEdit(ImePreEditEvent),
  ImePreEditCapture(ImePreEditEvent),
  /// Firing the wheel event when the user rotates a wheel button on a pointing
  /// device (typically a mouse).
  Wheel(WheelEvent),
  /// Same as `Wheel` but emit in capture phase.
  WheelCapture(WheelEvent),
  Chars(CharsEvent),
  CharsCapture(CharsEvent),
  /// The `KeyDown` event is fired when a key is pressed.
  KeyDown(KeyboardEvent),
  /// The `KeyDownCapture` event is same as `KeyDown` but emit in capture phase.
  KeyDownCapture(KeyboardEvent),
  /// The `KeyUp` event is fired when a key is released.
  KeyUp(KeyboardEvent),
  /// The `KeyUpCapture` event is same as `KeyUp` but emit in capture phase.
  KeyUpCapture(KeyboardEvent),
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  Focus(FocusEvent),
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  Blur(FocusEvent),
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  FocusIn(FocusEvent),
  /// The focusin capture event fires when an widget is about to receive focus.
  /// The main difference between this event and focusin is that focusin emit in
  /// bubbles phase but this event emit in capture phase.
  FocusInCapture(FocusEvent),
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  FocusOut(FocusEvent),
  /// The focusout capture event fires when an widget is about to lose focus.
  /// The main difference between this event and focusout is that focusout emit
  /// in bubbles phase but this event emit in capture phase.
  FocusOutCapture(FocusEvent),
  /// Custom event.
  CustomEvent(CustomEvent<dyn Any>),
}

impl Deref for Event {
  type Target = CommonEvent;

  fn deref(&self) -> &Self::Target {
    match self {
      Event::Mounted(e) | Event::PerformedLayout(e) | Event::Disposed(e) => e,
      Event::Focus(e)
      | Event::Blur(e)
      | Event::FocusIn(e)
      | Event::FocusInCapture(e)
      | Event::FocusOut(e)
      | Event::FocusOutCapture(e) => e,
      Event::PointerDown(e)
      | Event::PointerDownCapture(e)
      | Event::PointerUp(e)
      | Event::PointerUpCapture(e)
      | Event::PointerMove(e)
      | Event::PointerMoveCapture(e)
      | Event::PointerCancelCapture(e)
      | Event::PointerCancel(e)
      | Event::PointerEnter(e)
      | Event::PointerLeave(e)
      | Event::Tap(e)
      | Event::TapCapture(e) => e,
      Event::ImePreEdit(e) | Event::ImePreEditCapture(e) => e,
      Event::Wheel(e) | Event::WheelCapture(e) => e,
      Event::Chars(e) | Event::CharsCapture(e) => e,
      Event::KeyDown(e) | Event::KeyDownCapture(e) | Event::KeyUp(e) | Event::KeyUpCapture(e) => e,
      Event::CustomEvent(e) => e,
    }
  }
}

impl DerefMut for Event {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      Event::Mounted(e) | Event::PerformedLayout(e) | Event::Disposed(e) => e,
      Event::Focus(e)
      | Event::Blur(e)
      | Event::FocusIn(e)
      | Event::FocusInCapture(e)
      | Event::FocusOut(e)
      | Event::FocusOutCapture(e) => e,
      Event::PointerDown(e)
      | Event::PointerDownCapture(e)
      | Event::PointerUp(e)
      | Event::PointerUpCapture(e)
      | Event::PointerMove(e)
      | Event::PointerMoveCapture(e)
      | Event::PointerCancel(e)
      | Event::PointerCancelCapture(e)
      | Event::PointerEnter(e)
      | Event::PointerLeave(e)
      | Event::Tap(e)
      | Event::TapCapture(e) => e,
      Event::ImePreEdit(e) | Event::ImePreEditCapture(e) => e,
      Event::Wheel(e) | Event::WheelCapture(e) => e,
      Event::Chars(e) | Event::CharsCapture(e) => e,
      Event::KeyDown(e) | Event::KeyDownCapture(e) | Event::KeyUp(e) | Event::KeyUpCapture(e) => e,
      Event::CustomEvent(e) => e,
    }
  }
}

impl Event {
  pub(crate) fn flags(&self) -> MixFlags {
    match self {
      Event::Mounted(_) | Event::PerformedLayout(_) | Event::Disposed(_) => MixFlags::Lifecycle,
      Event::PointerDown(_)
      | Event::PointerDownCapture(_)
      | Event::PointerUp(_)
      | Event::PointerUpCapture(_)
      | Event::PointerMove(_)
      | Event::PointerMoveCapture(_)
      | Event::PointerCancel(_)
      | Event::PointerCancelCapture(_)
      | Event::PointerEnter(_)
      | Event::PointerLeave(_)
      | Event::Tap(_)
      | Event::TapCapture(_) => MixFlags::Pointer,
      Event::Wheel(_) | Event::WheelCapture(_) => MixFlags::Wheel,
      Event::ImePreEdit(_)
      | Event::ImePreEditCapture(_)
      | Event::Chars(_)
      | Event::CharsCapture(_)
      | Event::KeyDown(_)
      | Event::KeyDownCapture(_)
      | Event::KeyUp(_)
      | Event::KeyUpCapture(_) => MixFlags::KeyBoard,
      Event::Focus(_) | Event::Blur(_) => MixFlags::Focus,
      Event::FocusIn(_)
      | Event::FocusInCapture(_)
      | Event::FocusOut(_)
      | Event::FocusOutCapture(_) => MixFlags::FocusInOut,
      Event::CustomEvent(_) => MixFlags::Customs,
    }
  }
}

impl std::fmt::Debug for CommonEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.id)
      .field("current_target", &self.id)
      .field("is_propagation", &self.propagation)
      .finish()
  }
}

impl CommonEvent {
  /// Create a new common event.
  ///
  /// Although the `dispatcher` is contained in the `wnd`, we still need to pass
  /// it because in most case the event create in a environment that the
  /// `Dispatcher` already borrowed.
  pub(crate) fn new(target: WidgetId, tree: NonNull<WidgetTree>) -> Self {
    Self {
      target,
      id: target,
      propagation: true,
      prevent_default: false,
      provider_ctx: ProviderCtx::collect_from(target, unsafe { tree.as_ref() }),
      tree,
    }
  }

  pub(crate) fn bubble_to_parent(&mut self, id: WidgetId) -> bool {
    if let Some(parent) = id.parent(self.tree()) {
      self.provider_ctx.pop_providers_for(id);
      self.id = parent;
      true
    } else {
      false
    }
  }

  pub(crate) fn capture_to_child(&mut self, id: WidgetId, buffer: &mut SmallVec<[QueryHandle; 1]>) {
    let tree = unsafe { self.tree.as_ref() };
    self
      .provider_ctx
      .push_providers_for(id, tree, buffer);
    self.id = id;
  }

  fn pick_info<R>(&self, f: impl FnOnce(&DispatchInfo) -> R) -> R {
    f(&self.window().dispatcher.borrow().info)
  }
}

impl WidgetCtxImpl for CommonEvent {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  fn tree(&self) -> &WidgetTree { unsafe { self.tree.as_ref() } }
}

impl AsRef<ProviderCtx> for CommonEvent {
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl AsMut<ProviderCtx> for CommonEvent {
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}

impl AsRef<ProviderCtx> for Event {
  fn as_ref(&self) -> &ProviderCtx { self.deref().as_ref() }
}

impl AsMut<ProviderCtx> for Event {
  fn as_mut(&mut self) -> &mut ProviderCtx { self.deref_mut().as_mut() }
}
