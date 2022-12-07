use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

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
