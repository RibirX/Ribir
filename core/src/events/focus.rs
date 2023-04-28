use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_for_listener,
  impl_compose_child_with_focus_for_listener, impl_listener, impl_query_self_only, prelude::*,
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
pub struct FocusOutListener {
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(builtin, convert=custom)]
  on_focus_out: MutRefItemSubject<'static, FocusEvent, Infallible>,
}

impl_listener!(
  FocusListener,
  FocusListenerDeclarer,
  on_focus,
  FocusEvent,
  focus_stream
);
impl_compose_child_with_focus_for_listener!(FocusListener);

impl_listener!(
  BlurListener,
  BlurListenerDeclarer,
  on_blur,
  FocusEvent,
  blur_stream
);
impl_compose_child_with_focus_for_listener!(BlurListener);

impl_listener!(
  FocusInListener,
  FocusInListenerDeclarer,
  on_focus_in,
  FocusEvent,
  focus_in_stream
);
impl_compose_child_for_listener!(FocusInListener);

impl_listener!(
  FocusOutListener,
  FocusOutListenerDeclarer,
  on_focus_out,
  FocusEvent,
  focus_out_stream
);
impl_compose_child_for_listener!(FocusOutListener);
