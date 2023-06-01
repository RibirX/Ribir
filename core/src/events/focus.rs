use std::convert::Infallible;

use crate::{
  impl_compose_child_for_listener, impl_compose_child_with_focus_for_listener, impl_listener,
  impl_listener_and_compose_child, impl_listener_and_compose_child_with_focus,
  impl_query_self_only, prelude::*,
};

pub type FocusEvent = EventCommon;

#[derive(Declare)]
pub struct FocusListener {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  on_focus: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

#[derive(Declare)]
pub struct BlurListener {
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  #[declare(builtin, convert=custom)]
  on_blur: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

#[derive(Declare)]
pub struct FocusInListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  on_focus_in: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

#[derive(Declare)]
pub struct FocusInCaptureListener {
  #[declare(builtin, convert=custom)]
  on_focus_in_capture: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

#[derive(Declare)]
pub struct FocusOutListener {
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(builtin, convert=custom)]
  on_focus_out: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

#[derive(Declare)]
pub struct FocusOutCaptureListener {
  #[declare(builtin, convert=custom)]
  on_focus_out_capture: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

impl_listener_and_compose_child_with_focus!(
  FocusListener,
  FocusListenerDeclarer,
  on_focus,
  FocusEvent,
  focus_stream
);

impl_listener_and_compose_child_with_focus!(
  BlurListener,
  BlurListenerDeclarer,
  on_blur,
  FocusEvent,
  blur_stream
);

impl_listener_and_compose_child!(
  FocusInListener,
  FocusInListenerDeclarer,
  on_focus_in,
  FocusEvent,
  focus_in_stream
);

impl_listener_and_compose_child!(
  FocusInCaptureListener,
  FocusInCaptureListenerDeclarer,
  on_focus_in_capture,
  FocusEvent,
  focus_in_capture_stream
);

impl_listener_and_compose_child!(
  FocusOutListener,
  FocusOutListenerDeclarer,
  on_focus_out,
  FocusEvent,
  focus_out_stream
);

impl_listener_and_compose_child!(
  FocusOutCaptureListener,
  FocusOutCaptureListenerDeclarer,
  on_focus_out_capture,
  FocusEvent,
  focus_out_capture_stream
);
