use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;

#[derive(Default, Declare)]
pub struct FocusListener {
  #[declare(default, builtin, convert=custom)]
  pub focus: Callback,
  #[declare(default, builtin, convert=custom)]
  pub blur: Callback,
}

#[derive(Declare)]
pub struct FocusInOutListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(default, builtin, convert=custom)]
  pub focus_in: Callback,

  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(default, builtin, convert=custom)]
  pub focus_out: Callback,
}

type Callback = RefCell<Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>>;

pub type FocusEvent = EventCommon;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusEventType {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  Focus,
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  Blur,
}

impl ComposeChild for FocusListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for FocusListener {
  impl_query_self_only!();
}

macro_rules! dispatch_event {
  ($callback: expr, $event: ident) => {
    let mut callback = $callback.borrow_mut();
    if let Some(callback) = callback.as_mut() {
      callback($event)
    }
  };
}

impl FocusListener {
  #[inline]
  pub fn dispatch_focus(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus, event);
  }

  pub fn dispatch_blur(&self, event: &mut FocusEvent) {
    dispatch_event!(self.blur, event);
  }
}

impl FocusListenerDeclarer {
  #[inline]
  pub fn focus(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus = Some(into_callback(f));
    self
  }

  #[inline]
  pub fn blur(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.blur = Some(into_callback(f));
    self
  }
}

fn into_callback(f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Callback {
  RefCell::new(Some(Box::new(f)))
}

impl FocusListener {
  #[inline]
  pub fn set_declare_focus(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus = into_callback(f);
  }

  #[inline]
  pub fn set_declare_blur(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.blur = into_callback(f);
  }
}

impl Query for FocusInOutListener {
  impl_query_self_only!();
}

impl ComposeChild for FocusInOutListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl FocusInOutListener {
  #[inline]
  pub fn dispatch_focus_in(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus_in, event);
  }

  pub fn dispatch_focus_out(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus_out, event);
  }
}

impl FocusInOutListenerDeclarer {
  #[inline]
  pub fn focus_in(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus_in = Some(into_callback(f));
    self
  }

  #[inline]
  pub fn focus_out(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus_out = Some(into_callback(f));
    self
  }
}
