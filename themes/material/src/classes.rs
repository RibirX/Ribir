use ribir_core::prelude::Classes;

mod avatar_cls;
mod badge_cls;
mod buttons_cls;
mod checkbox_cls;
mod disabled_cls;
mod divider_cls;
mod input_cls;
mod list_cls;
mod menu_cls;
mod navigation_rail_cls;
mod progress_cls;
mod radio_cls;
mod scrollbar_cls;
mod slider_cls;
mod switch_cls;
mod tabs_cls;
mod tooltips_cls;

pub fn initd_classes() -> Classes {
  let mut classes = Classes::default();

  buttons_cls::init(&mut classes);
  scrollbar_cls::init(&mut classes);
  radio_cls::init(&mut classes);
  progress_cls::init(&mut classes);
  checkbox_cls::init(&mut classes);
  tooltips_cls::init(&mut classes);
  slider_cls::init(&mut classes);
  input_cls::init(&mut classes);
  divider_cls::init(&mut classes);
  menu_cls::init(&mut classes);
  tabs_cls::init(&mut classes);
  disabled_cls::init(&mut classes);
  avatar_cls::init(&mut classes);
  list_cls::init(&mut classes);
  switch_cls::init(&mut classes);
  badge_cls::init(&mut classes);
  navigation_rail_cls::init(&mut classes);

  classes
}
