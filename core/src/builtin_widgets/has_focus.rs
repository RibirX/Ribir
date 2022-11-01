use crate::prelude::*;
#[derive(PartialEq, Clone, Declare)]
pub struct HasFocus {
  #[declare(skip, default)]
  focused: bool,
}

impl HasFocus {
  pub fn has_focus(&self) -> bool { self.focused }
}

impl ComposeChild for HasFocus {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      track {this: this.into_stateful()}
      ExprWidget {
        expr: child,
        focus: move|_| this.focused = true,
        blur: move |_| this.focused = false,
      }
    }
  }
}
