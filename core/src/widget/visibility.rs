use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Visibility {
  #[declare(builtin)]
  visible: bool,
}

impl ComposeSingleChild for Visibility {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
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

