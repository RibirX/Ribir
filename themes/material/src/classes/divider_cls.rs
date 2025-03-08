use ribir_core::prelude::*;
use ribir_widgets::prelude::{
  HORIZONTAL_DIVIDER, HORIZONTAL_DIVIDER_INDENT_BOTH, HORIZONTAL_DIVIDER_INDENT_END,
  HORIZONTAL_DIVIDER_INDENT_START, VERTICAL_DIVIDER, VERTICAL_DIVIDER_INDENT_BOTH,
  VERTICAL_DIVIDER_INDENT_END, VERTICAL_DIVIDER_INDENT_START,
};

const THICKNESS: f32 = 1.;
const MARGIN_HORIZONTAL: f32 = 16.;
const MARGIN_VERTICAL: f32 = 8.;

named_style_class!(horizontal_base => {
  clamp: BoxClamp::fixed_size(Size::new(f32::INFINITY, THICKNESS)),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});

named_style_class!(vertical_base => {
  clamp: BoxClamp::fixed_size(Size::new(THICKNESS, f32::INFINITY)),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});
pub(super) fn init(classes: &mut Classes) {
  classes.insert(HORIZONTAL_DIVIDER, horizontal_base);

  classes.insert(HORIZONTAL_DIVIDER_INDENT_START, multi_class! {
    horizontal_base,
    style_class!{ margin: EdgeInsets::only_left(MARGIN_HORIZONTAL) }
  });

  classes.insert(HORIZONTAL_DIVIDER_INDENT_END, multi_class! {
    horizontal_base,
    style_class!{ margin: EdgeInsets::only_right(MARGIN_HORIZONTAL) }
  });

  classes.insert(HORIZONTAL_DIVIDER_INDENT_BOTH, multi_class! {
    horizontal_base,
    style_class!{ margin: EdgeInsets::horizontal(MARGIN_HORIZONTAL) }
  });

  classes.insert(HORIZONTAL_DIVIDER_INDENT_BOTH, multi_class! {
    horizontal_base,
    style_class!{ margin: EdgeInsets::horizontal(MARGIN_HORIZONTAL) }
  });

  classes.insert(VERTICAL_DIVIDER, vertical_base);

  classes.insert(VERTICAL_DIVIDER_INDENT_START, multi_class! {
    vertical_base,
    style_class! { margin: EdgeInsets::only_top(MARGIN_VERTICAL)}
  });

  classes.insert(VERTICAL_DIVIDER_INDENT_END, multi_class! {
    vertical_base,
    style_class! { margin: EdgeInsets::only_bottom(MARGIN_VERTICAL)}
  });

  classes.insert(VERTICAL_DIVIDER_INDENT_BOTH, multi_class! {
    vertical_base,
    style_class! { margin: EdgeInsets::vertical(MARGIN_VERTICAL)}
  });
}
