use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

type Callback = Rc<RefCell<Box<dyn FnMut(&mut FocusEvent)>>>;
pub type FocusEvent = EventCommon;

#[derive(Declare)]
pub struct FocusListener {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  focus: Callback,
  #[declare(skip)]
  pub focus_stream: LocalSubject<'static, Rc<RefCell<FocusEvent>>, ()>,
}

#[derive(Declare)]
pub struct BlurListener {
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  #[declare(builtin, convert=custom)]
  blur: Callback,
  #[declare(skip)]
  pub blur_stream: LocalSubject<'static, Rc<RefCell<FocusEvent>>, ()>,
}

#[derive(Declare)]
pub struct FocusInListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(builtin, convert=custom)]
  focus_in: Callback,
  #[declare(skip)]
  pub focus_in_stream: LocalSubject<'static, Rc<RefCell<FocusEvent>>, ()>,
}

#[derive(Declare)]
pub struct FocusOutListener {
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(builtin, convert=custom)]
  focus_out: Callback,
  #[declare(skip)]
  pub focus_out_stream: LocalSubject<'static, Rc<RefCell<FocusEvent>>, ()>,
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

macro_rules! impl_focus_listener {
  ($({$name: ident, $field: ident, $event_ty: ty},)*) => {
    $(
      declare_builtin_event_field!($name, $field, $event_ty);

      impl Query for $name {
        impl_query_self_only!();
      }
      impl_event_stream_dispatch!($name, $field, $event_ty);
    )*
  };
}

impl_focus_listener!(
  { FocusListener, focus, FocusEvent},
  { BlurListener,  blur, FocusEvent},
  { FocusInListener, focus_in, FocusEvent},
  { FocusOutListener, focus_out, FocusEvent},
);
