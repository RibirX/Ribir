use rxrust::prelude::*;
use std::convert::Infallible;

use crate::{
  impl_all_event, impl_compose_child_for_listener, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_multi_event_listener, impl_query_self_only, prelude::*,
};

pub type FocusEvent = CommonEvent;
pub type FocusSubject = MutRefItemSubject<'static, AllFocus, Infallible>;

impl_multi_event_listener! {
  "The listener use to fire and listen focus events.",
  Focus,
  "The focus event fires when an widget has received focus. The main \
  difference between this event and focusin is that focusin bubbles while\
  focus does not.",
  Focus,
  "The blur event fires when an widget has lost focus. The main difference \
  between this event and focusout is that focusout bubbles while blur does not.",
  Blur
}
impl_compose_child_with_focus_for_listener!(FocusListener);

pub type FocusBubbleEvent = CommonEvent;
pub type FocusBubbleSubject = MutRefItemSubject<'static, AllFocusBubble, Infallible>;

impl_multi_event_listener! {
  "The listener use to fire and listen focusin and focusout events.",
  FocusBubble,
  "The focusin event fires when an widget is about to receive focus. The main \
  difference between this event and focus is that focusin bubbles while \
  focus does not.",
  FocusIn,
  "The focusin capture event fires when an widget is about to receive focus. The main \
  difference between this event and focusin is that focusin emit in bubbles phase \
  but this event emit in capture phase.",
  FocusInCapture,
  "The focusout event fires when an widget is about to lose focus. The main \
  difference between this event and blur is that focusout bubbles while blur \
  does not.",
  FocusOut,
  "The focusout capture event fires when an widget is about to lose focus. The main \
  difference between this event and focusout is that focusout emit in bubbles phase \
  but this event emit in capture phase.",
  FocusOutCapture
}
impl_compose_child_for_listener!(FocusBubbleListener);
