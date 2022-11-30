use crate::prelude::*;

/// Widget keep the pointer press state of its child. As a builtin widget, user
/// can call `pointer_pressed` method to get the pressed state of a widget.
#[derive(Declare)]
pub struct PointerPressed {
  #[declare(skip, builtin)]
  pointer_pressed: bool,
}

impl PointerPressed {
  // return if its child widget is pressed.
  #[inline]
  pub fn pointer_pressed(&self) -> bool { self.pointer_pressed }
}

impl ComposeChild for PointerPressed {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_stateful()}
      DynWidget {
        dyns: child,
        pointer_down: move|_| this.pointer_pressed = true,
        pointer_up: move |_| this.pointer_pressed = false,
      }
    }
  }
}
