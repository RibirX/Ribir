use crate::prelude::*;
use rxrust::prelude::*;
use std::rc::Rc;

/// Focus attr attach to widget to support get ability to focus in.
pub type FocusListener<W> = AttrWidget<W, FocusAttr>;

#[derive(Debug, Default)]
pub struct FocusAttr {
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
  pub tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial
  /// focus.
  pub auto_focus: bool,
  subject: LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()>,
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

impl<W: Widget> FocusListener<W> {
  pub fn from_widget<A: AttachAttr<W = W>>(
    widget: A,
    auto_focus: Option<bool>,
    tab_index: Option<i16>,
  ) -> Self {
    let (major, others, widget) = widget.take_attr();
    let mut major: FocusAttr = major.unwrap_or_default();
    major.tab_index = tab_index.unwrap_or(0);
    major.auto_focus = auto_focus.unwrap_or(false);
    FocusListener { major, widget, others }
  }
  #[inline]
  pub fn focus_event_observable(
    &self,
  ) -> LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()> {
    self.major.focus_event_observable()
  }

  pub fn listen_on<H: FnMut(&FocusEvent) + 'static>(
    &self,
    event_type: FocusEventType,
    mut handler: H,
  ) -> SubscriptionWrapper<LocalSubscription> {
    self
      .focus_event_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event))
  }

  #[inline]
  pub fn is_auto_focus(&self) -> bool { self.major.auto_focus }

  #[inline]
  pub fn tab_index(&self) -> i16 { self.major.tab_index }
}

impl FocusAttr {
  #[inline]
  pub fn focus_event_observable(
    &self,
  ) -> LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()> {
    self.subject.clone()
  }
}
