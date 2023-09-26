use crate::prelude::*;

/// Widget keep the pointer press state of its child. As a builtin widget, user
/// can call `pointer_pressed` method to get the pressed state of a widget.
#[derive(Declare2)]
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
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @ $child {
        on_pointer_down: move|_| $this.write().pointer_pressed = true,
        on_pointer_up: move |_| $this.write().pointer_pressed = false,
      }
    }
  }
}
