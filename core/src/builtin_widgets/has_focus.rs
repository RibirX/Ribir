use crate::prelude::*;
#[derive(PartialEq, Clone, Default)]
pub struct HasFocus {
  focused: bool,
}

impl HasFocus {
  pub fn has_focus(&self) -> bool { self.focused }
}

impl Declare for HasFocus {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for HasFocus {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      @ $child {
        on_focus_in: move|_| $this.write().focused = true,
        on_focus_out: move |_| $this.write().focused = false,
      }
    }
    .into_widget()
  }
}
