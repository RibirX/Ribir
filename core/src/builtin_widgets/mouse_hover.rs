use crate::prelude::*;

#[derive(PartialEq, Clone, Declare2)]
pub struct MouseHover {
  #[declare(skip, default)]
  hover: bool,
}

impl MouseHover {
  pub fn mouse_hover(&self) -> bool { self.hover }
}

impl ComposeChild for MouseHover {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @ $child {
        on_pointer_enter: move |_| $this.write().hover = true,
        on_pointer_leave: move |_| $this.write().hover = false,
      }
    }
  }
}
