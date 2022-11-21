use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;

// todo: split focus listener, and auto add focus node when listen on key/char.
/// Focus attr attach to widget to support get ability to focus in.
#[derive(Default, Declare)]
pub struct FocusListener {
  /// Indicates that `widget` can be focused, and where it participates in
  /// sequential keyboard navigation (usually with the Tab key, hence the name.
  ///
  /// It accepts an integer as a value, with different results depending on the
  /// integer's value:
  /// - A negative value (usually -1) means that the widget is not reachable via
  ///   sequential keyboard navigation, but could be focused with API or
  ///   visually by clicking with the mouse.
  /// - Zero means that the element should be focusable in sequential keyboard
  ///   navigation, after any positive tab_index values and its order is defined
  ///   by the tree's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the tree source. The maximum value for tab_index is 32767.
  ///   If not specified, it takes the default value 0.
  #[declare(default, builtin)]
  pub tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial
  /// focus.
  #[declare(default, builtin)]
  pub auto_focus: bool,
  #[declare(default, builtin, convert=custom)]
  pub focus: Callback,
  #[declare(default, builtin, convert=custom)]
  pub blur: Callback,
  #[declare(default, builtin, convert=custom)]
  pub focus_in: Callback,
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
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  FocusIn,
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  FocusOut,
}

impl ComposeChild for FocusListener {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for FocusListener {
  impl_query_self_only!();
}

impl FocusListener {
  #[inline]
  pub fn dispatch_event(&self, event_type: FocusEventType, event: &mut FocusEvent) {
    let mut callback = match event_type {
      FocusEventType::Focus => self.focus.borrow_mut(),
      FocusEventType::Blur => self.blur.borrow_mut(),
      FocusEventType::FocusIn => self.focus_in.borrow_mut(),
      FocusEventType::FocusOut => self.focus_out.borrow_mut(),
    };
    if let Some(callback) = callback.as_mut() {
      callback(event)
    }
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

  #[inline]
  pub fn set_declare_focus_in(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus_in = into_callback(f);
  }

  #[inline]
  pub fn set_declare_focus_out(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus_out = into_callback(f);
  }
}
