use crate::{impl_query_self_only, prelude::*};

/// Focus attr attach to widget to support get ability to focus in.
#[derive(Default, Declare, SingleChild)]
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
  #[declare(default, builtin, custom_convert)]
  on_focus: Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>,
  #[declare(default, builtin, custom_convert)]
  on_blur: Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>,
  #[declare(default, builtin, custom_convert)]
  on_focus_in: Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>,
  #[declare(default, builtin, custom_convert)]
  on_focus_out: Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>,
}

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

// FocusListener must be a widget node to avoid two FocusListener in same
// widget.
impl Render for FocusListener {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map(|c| ctx.perform_child_layout(c, clamp))
      .unwrap_or_default()
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for FocusListener {
  impl_query_self_only!();
}

impl FocusListener {
  #[inline]
  pub fn dispatch_event(&mut self, event_type: FocusEventType, event: &mut FocusEvent) {
    let callback = match event_type {
      FocusEventType::Focus => self.on_focus.as_mut(),
      FocusEventType::Blur => self.on_blur.as_mut(),
      FocusEventType::FocusIn => self.on_focus_in.as_mut(),
      FocusEventType::FocusOut => self.on_focus_out.as_mut(),
    };
    if let Some(callback) = callback {
      callback(event)
    }
  }
}

impl FocusListenerBuilder {
  #[inline]
  pub fn on_focus_convert(
    f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static,
  ) -> Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>> {
    Some(Box::new(f))
  }

  #[inline]
  pub fn on_blur_convert(
    f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static,
  ) -> Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>> {
    Some(Box::new(f))
  }

  #[inline]
  pub fn on_focus_in_convert(
    f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static,
  ) -> Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>> {
    Some(Box::new(f))
  }

  #[inline]
  pub fn on_focus_out_convert(
    f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static,
  ) -> Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>> {
    Some(Box::new(f))
  }
}
