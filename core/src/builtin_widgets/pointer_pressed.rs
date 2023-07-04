use crate::prelude::*;

/// Widget keep the pointer press state of its child. As a builtin widget, user
/// can call `pointer_pressed` method to get the pressed state of a widget.
#[derive(Declare, Declare2)]
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
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_writable()}
      DynWidget {
        dyns: child,
        on_pointer_down: move|_| this.pointer_pressed = true,
        on_pointer_up: move |_| this.pointer_pressed = false,
      }
    }
    .into()
  }
}
