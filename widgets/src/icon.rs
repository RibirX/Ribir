use ribir_core::prelude::*;

use crate::layout::SizedBox;

/// Widget that let child paint as a icon with special size. Unlike icon in
/// classic frameworks, it's not draw anything and not require you to
/// provide image or font fot it to draw, it just center align and fit size of
/// its child. So you can declare any widget as its child to display as a icon.
#[derive(Declare, Default, Clone, Copy)]
pub struct Icon {
  #[declare(default = IconSize::of(ctx!()).small)]
  pub size: Size,
}

impl ComposeChild for Icon {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @SizedBox {
        size: pipe!($this.size),
        @ $child {
          box_fit: BoxFit::Contain,
          h_align: HAlign::Center,
          v_align: VAlign::Center,
        }
      }
    }
  }
}

macro_rules! define_fixed_size_icon {
  ($($name: ident, $field: ident),*) => {
    $(
      #[derive(Declare, Default, Clone, Copy)]
      pub struct $name;

      impl ComposeChild for $name {
        type Child = Widget;
        fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child)
          -> impl WidgetBuilder
        {
          fn_widget! {
            let icon = @Icon { size: IconSize::of(ctx!()).$field };
            @ $icon { @ { child } }
          }
        }
      }
    )*
  };
}

define_fixed_size_icon!(TinyIcon, tiny);
define_fixed_size_icon!(SmallIcon, small);
define_fixed_size_icon!(MediumIcon, medium);
define_fixed_size_icon!(LargeIcon, large);
define_fixed_size_icon!(HugeIcon, huge);
