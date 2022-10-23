use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Visibility {
  #[declare(builtin)]
  pub visible: bool,
}

impl ComposeChild for Visibility {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget_try_track! {
      try_track { this }
      Offstage {
        offstage: !this.visible,
        IgnorePointer {
          ignore: !this.visible,
          ExprWidget { expr: child }
        }
      }
    }
  }
}
