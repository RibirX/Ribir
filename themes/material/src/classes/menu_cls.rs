use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    MENU,
    style_class! {
      background: Palette::of(BuildCtx::get()).surface_container(),
      clamp: BoxClamp::min_width(112.).with_max_width(280.),
      radius: Radius::all(4.),
    },
  );
  classes.insert(
    MENU_ITEM,
    style_class! {
      clamp: BoxClamp::fixed_height(48.),
    },
  );
  classes.insert(
    MENU_ITEM_SELECTED,
    style_class! {
      clamp: BoxClamp::fixed_height(48.),
      background: Palette::of(BuildCtx::get()).secondary_container(),
    },
  );
  classes.insert(
    MENU_ITEM_LABEL,
    style_class! {
      foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
      padding: EdgeInsets::horizontal(12.),
      text_line_height: 20.,
      font_size: 14.,
    },
  );
  classes.insert(
    MENU_ITEM_LEADING,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::new(24., 24.)),
      margin: EdgeInsets::only_left(12.),
    },
  );
  classes.insert(
    MENU_ITEM_HINT_TEXT,
    style_class! {
      padding: EdgeInsets::only_right(12.),
    },
  );
  classes.insert(
    MENU_ITEM_TRAILING,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::new(24., 24.)),
      margin: EdgeInsets::only_right(12.),
    },
  );
  classes.insert(
    MENU_DIVIDER,
    style_class! {
      clamp: BoxClamp::fixed_height(1.).with_fixed_width(f32::INFINITY),
      background: Palette::of(BuildCtx::get()).surface_variant(),
      margin: EdgeInsets::vertical(8.),
    },
  );
}
