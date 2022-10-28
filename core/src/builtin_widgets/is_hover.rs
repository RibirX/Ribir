use crate::prelude::*;

#[derive(PartialEq, Clone, Declare)]
pub struct IsHover {
  #[declare(skip, default)]
  hover: bool,
}

impl IsHover {
  pub fn is_hover(&self) -> bool { self.hover }
}

impl ComposeChild for IsHover {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let this = this.into_stateful();

    widget! {
      track {this}
      ExprWidget {
        expr: child,
        pointer_enter: move |_| this.hover = true,
        pointer_leave: move |_| this.hover = false,
      }
    }
  }
}
