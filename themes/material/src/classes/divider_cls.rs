use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

const THICKNESS: f32 = 1.;

named_style_impl!(horizontal_base => {
  clamp: BoxClamp::EXPAND_X.with_fixed_height(THICKNESS),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});

named_style_impl!(vertical_base => {
  clamp: BoxClamp::EXPAND_Y.with_fixed_width(THICKNESS),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});
pub(super) fn init(classes: &mut Classes) {
  classes.insert(HORIZONTAL_DIVIDER, horizontal_base);

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_START,
    class_multi_impl! {
      horizontal_base,
      style_class!{ margin: md::EDGES_LEFT_16 }
    },
  );

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_END,
    class_multi_impl! {
      horizontal_base,
      style_class!{ margin: md::EDGES_RIGHT_16 }
    },
  );

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_BOTH,
    class_multi_impl! {
      horizontal_base,
      style_class!{ margin: md::EDGES_HOR_16 }
    },
  );

  classes.insert(VERTICAL_DIVIDER, vertical_base);

  classes.insert(
    VERTICAL_DIVIDER_INDENT_START,
    class_multi_impl! {
      vertical_base,
      style_class! { margin: md::EDGES_TOP_8 }
    },
  );

  classes.insert(
    VERTICAL_DIVIDER_INDENT_END,
    class_multi_impl! {
      vertical_base,
      style_class! { margin: md::EDGES_BOTTOM_8}
    },
  );

  classes.insert(
    VERTICAL_DIVIDER_INDENT_BOTH,
    class_multi_impl! {
      vertical_base,
      style_class! { margin: md::EDGES_VER_8 }
    },
  );
}
