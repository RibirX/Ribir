use crate::prelude::*;

#[derive(PartialEq, Clone, Default)]
pub struct MouseHover {
  hover: bool,
}

impl MouseHover {
  pub fn mouse_hover(&self) -> bool { self.hover }
}

impl Declare for MouseHover {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for MouseHover {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      @ $child {
        on_pointer_enter: move |_| $this.write().hover = true,
        on_pointer_leave: move |_| $this.write().hover = false,
      }
    }
    .into_widget()
  }
}
