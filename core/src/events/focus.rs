use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;

type Callback = RefCell<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>;
pub type FocusEvent = EventCommon;

#[derive(Declare)]
pub struct FocusListener {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut FocusEvent),
    wrap_fn = RefCell::new)
  )]
  pub on_focus: Callback,
}

#[derive(Declare)]
pub struct BlurListener {
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut FocusEvent),
    wrap_fn = RefCell::new)
  )]
  pub on_blur: Callback,
}

#[derive(Declare)]
pub struct FocusInListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut FocusEvent),
    wrap_fn = RefCell::new)
  )]
  pub on_focus_in: Callback,
}

#[derive(Declare)]
pub struct FocusOutListener {
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut FocusEvent),
    wrap_fn = RefCell::new)
  )]
  pub on_focus_out: Callback,
}

impl ComposeChild for FocusListener {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let widget = dynamic_compose_focus_node(child);
    compose_child_as_data_widget(widget, this)
  }
}

impl ComposeChild for BlurListener {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let widget = dynamic_compose_focus_node(child);
    compose_child_as_data_widget(widget, this)
  }
}

impl ComposeChild for FocusInListener {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl ComposeChild for FocusOutListener {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

macro_rules! impl_common_focus_ability {
  ($({$struct: ty, $builder: ty, $field: ident},)*) => {
    $(
    impl $struct {
      #[inline]
      pub fn dispatch(&self, event: &mut FocusEvent) {
        let mut callback = self.$field.borrow_mut();
        callback.as_mut()(event);

      }
    }
    impl Query for $struct {
      impl_query_self_only!();
    }
    )*
  };
}

impl_common_focus_ability!(
  { FocusListener, FocusListenerDeclarer, on_focus},
  { BlurListener, BlurListenerDeclarer, on_blur},
  { FocusInListener, FocusInListenerDeclarer, on_focus_in},
  { FocusOutListener, FocusOutListenerDeclarer, on_focus_out},
);
