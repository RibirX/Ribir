use crate::prelude::*;

#[derive(PartialEq, Clone, Declare)]
pub struct MouseHover {
  #[declare(skip, default)]
  hover: bool,
}

impl MouseHover {
  pub fn mouse_hover(&self) -> bool { self.hover }
}

impl ComposeChild for MouseHover {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states {this: this.into_writable()}
      DynWidget {
        dyns: child,
        pointer_enter: move |_| this.hover = true,
        pointer_leave: move |_| this.hover = false,
      }
    }
  }
}
