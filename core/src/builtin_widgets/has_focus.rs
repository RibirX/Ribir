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
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states {this: this.into_writable()}
      DynWidget {
        dyns: child,
        on_focus_in: move|_| this.focused = true,
        on_focus_out: move |_| this.focused = false,
      }
    }
  }
}
