use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;

type Callback = RefCell<Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>>;
pub type FocusEvent = EventCommon;

#[derive(Default, Declare)]
pub struct FocusListener {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  pub focus: Callback,
}

#[derive(Default, Declare)]
pub struct BlurListener {
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  #[declare(builtin, convert=custom)]
  pub blur: Callback,
}

#[derive(Declare)]
pub struct FocusInListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  pub focus_in: Callback,
}

#[derive(Declare)]
pub struct FocusOutListener {
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(builtin, convert=custom)]
  pub focus_out: Callback,
}

impl ComposeChild for FocusListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let widget = dynamic_compose_focus_node(child);
    compose_child_as_data_widget(widget, this)
  }
}

impl ComposeChild for BlurListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let widget = dynamic_compose_focus_node(child);
    compose_child_as_data_widget(widget, this)
  }
}

impl ComposeChild for FocusInListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl ComposeChild for FocusOutListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

fn into_callback(f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Callback {
  RefCell::new(Some(Box::new(f)))
}

macro_rules! impl_common_focus_ability {
  ($({$struct: ty, $builder: ty, $field: ident},)*) => {
    $(
    impl $struct {
      #[inline]
      pub fn dispatch(&self, event: &mut FocusEvent) {
        let mut callback = self.$field.borrow_mut();
        if let Some(callback) = callback.as_mut() {
          callback(event)
        }
      }
    }
    impl Query for $struct {
      impl_query_self_only!();
    }


    impl $builder {
      #[inline]
      pub fn $field(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
        self.$field = Some(into_callback(f));
        self
      }
    }
    )*
  };
}

impl_common_focus_ability!(
  { FocusListener, FocusListenerDeclarer, focus},
  { BlurListener, BlurListenerDeclarer, blur},
  { FocusInListener, FocusInListenerDeclarer, focus_in},
  { FocusOutListener, FocusOutListenerDeclarer, focus_out},
);

impl FocusListener {
  #[inline]
  pub fn set_declare_focus(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus = into_callback(f);
  }
}

impl BlurListener {
  #[inline]
  pub fn set_declare_blur(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.blur = into_callback(f);
  }
}

impl FocusInListener {
  #[inline]
  pub fn set_declare_focus_in(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus_in = into_callback(f);
  }
}

impl FocusOutListener {
  #[inline]
  pub fn set_declare_focus_out(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus_out = into_callback(f);
  }
}
