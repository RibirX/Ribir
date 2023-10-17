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
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @ $child {
        on_focus_in: move|_| $this.write().focused = true,
        on_focus_out: move |_| $this.write().focused = false,
      }
    }
  }
}
