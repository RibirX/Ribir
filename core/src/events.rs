use crate::{
  context::EventCtx,
  widget_tree::{WidgetId, WidgetTree},
};
use std::ptr::NonNull;

pub(crate) mod dispatcher;
mod pointers;
pub use pointers::*;
use ribir_geom::Point;
pub use winit::event::{ModifiersState, ScanCode, VirtualKeyCode};
mod focus;
pub use focus::*;
mod keyboard;
pub use keyboard::*;
mod character;
pub use character::*;
mod wheel;
pub use wheel::*;
pub(crate) mod focus_mgr;
mod listener_impl_helper;
use self::dispatcher::DispatchInfo;

/// Event itself contains the properties and methods which are common to all
/// events
#[derive(Clone)]
pub struct EventCommon {
  pub(crate) target: WidgetId,
  pub(crate) current_target: WidgetId,
  pub(crate) is_propagation: bool,
  pub(crate) prevent_default: bool,
  tree: NonNull<WidgetTree>,
  info: NonNull<DispatchInfo>,
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
  pub fn stop_propagation(&mut self) { self.is_propagation = true }
  /// Return it the event is canceled to bubble to parent.
  #[inline]
  pub fn is_propagation(&self) -> bool { self.is_propagation }
  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  #[inline]
  pub fn prevent_default(&mut self) { self.prevent_default = true; }

  /// Represents the current state of the keyboard modifiers
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.dispatch_info().modifiers() }

  /// Returns `true` if the shift key is pressed.
  pub fn with_shift_key(&self) -> bool { self.dispatch_info().modifiers().shift() }
  /// Returns `true` if the alt key is pressed.
  pub fn with_alt_key(&self) -> bool { self.dispatch_info().modifiers().alt() }
  /// Returns `true` if the ctrl key is pressed.
  pub fn with_ctrl_key(&self) -> bool { self.dispatch_info().modifiers().ctrl() }
  /// Returns `true` if the logo key is pressed.
  pub fn with_logo_key(&self) -> bool { self.dispatch_info().modifiers().logo() }

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
  pub fn global_pos(&self) -> Point { self.dispatch_info().global_pos() }

  /// The X, Y coordinate of the pointer in current target widget.
  #[inline]
  pub fn position(&self) -> Point {
    let tree = unsafe { self.tree.as_ref() };
    tree
      .store
      .map_from_global(self.global_pos(), self.current_target, &tree.arena)
  }

  /// The buttons being depressed (if any) in current state.
  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.dispatch_info().mouse_buttons() }

  /// The button number that was pressed (if applicable) when the mouse event
  /// was fired.
  #[inline]
  pub fn button_num(&self) -> u32 { self.mouse_buttons().bits().count_ones() }

  #[inline]
  pub fn context(&mut self) -> EventCtx {
    // Safety: framework promise event context only live in event dispatch and
    // there is no others to share `Context`.
    let WidgetTree { arena, store, wnd_ctx, .. } = unsafe { self.tree.as_ref() };
    EventCtx {
      id: self.current_target(),
      arena,
      store,
      wnd_ctx,
      info: self.dispatch_info_mut(),
    }
  }

  pub fn next_focus(&self) {
    let tree = unsafe { self.tree.as_ref() };
    tree.wnd_ctx.next_focus(&tree.arena);
  }

  pub fn prev_focus(&self) {
    let tree = unsafe { self.tree.as_ref() };
    tree.wnd_ctx.prev_focus(&tree.arena);
  }

  fn dispatch_info_mut(&mut self) -> &mut DispatchInfo {
    // Safety: framework promise `info` only live in event dispatch and
    // there is no others borrow `info`.
    unsafe { self.info.as_mut() }
  }

  fn dispatch_info(&self) -> &DispatchInfo {
    // Safety: framework promise `info` only live in event dispatch and
    // there is no others mutable borrow `info`.
    unsafe { self.info.as_ref() }
  }
}

pub trait EventListener {
  type Event: std::borrow::BorrowMut<EventCommon>;
  fn dispatch(&self, event: &mut Self::Event);
}

impl std::fmt::Debug for EventCommon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.target)
      .field("current_target", &self.current_target)
      .field("is_propagation", &self.is_propagation)
      .finish()
  }
}

impl EventCommon {
  pub(crate) fn new(target: WidgetId, tree: &WidgetTree, info: &DispatchInfo) -> Self {
    Self {
      target,
      current_target: target,
      is_propagation: <_>::default(),
      prevent_default: <_>::default(),
      tree: NonNull::from(tree),
      info: NonNull::from(info),
    }
  }
}
