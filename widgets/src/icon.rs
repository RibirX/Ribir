use crate::layout::SizedBox;
use ribir_core::prelude::*;

/// Widget that let child paint as a icon with special size. Unlike icon in
/// classic frameworks, it's not draw anything and not require you to provide
/// image or font fot it to draw, it just center align and fit size of its
/// child. So you can declare any widget as its child to display as a icon.
#[derive(Declare, Default, Clone)]
pub struct Icon {
  #[declare(default = IconSize::of(ctx).small)]
  pub size: Size,
}

impl ComposeChild for Icon {
  type Child = Widget;
  type Target = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Self::Target {
    widget! {
      states { this: this.into_readonly() }
      SizedBox {
        size: this.size,
        DynWidget {
          dyns: child,
          box_fit: BoxFit::Contain,
          h_align: HAlign::Center,
          v_align: VAlign::Center,
        }
      }
    }
    .into_widget()
  }
}
