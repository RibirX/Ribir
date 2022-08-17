use crate::prelude::*;

/// Widget that let child paint as a icon with special size. Unlike icon in
/// classic frameworks, it's not draw anything and not require you to provide
/// image or font fot it to draw, it just center align and fit size of its
/// child. So you can declare any widget as its child to display as a icon.
#[derive(Declare, Default, Clone)]
pub struct Icon {
  #[declare(default = IconSize::of(ctx).small)]
  pub size: Size,
}

impl ComposeSingleChild for Icon {
  fn compose_single_child(
    this: StateWidget<Self>,
    child: Option<Widget>,
    _: &mut BuildCtx,
  ) -> Widget {
    widget_try_track! {
      try_track { this }
      SizedBox {
        size: this.size,
        ExprWidget {
          expr: child,
          box_fit: BoxFit::Contain,
          h_align: HAlign::Center,
          v_align: VAlign::Center,
        }
      }
    }
  }
}
