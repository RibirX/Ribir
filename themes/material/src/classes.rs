use ribir_core::prelude::Classes;

mod buttons_cls;
mod checkbox_cls;
mod progress_cls;
mod radio_cls;
mod scrollbar_cls;
mod slider_cls;
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

  classes
}
