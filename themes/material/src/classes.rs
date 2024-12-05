use ribir_core::prelude::Classes;

mod checkbox_cls;
mod progress_cls;
mod radio_cls;
mod scrollbar_cls;

pub fn initd_classes() -> Classes {
  let mut classes = Classes::default();

  scrollbar_cls::init(&mut classes);
  radio_cls::init(&mut classes);
  progress_cls::init(&mut classes);
  checkbox_cls::init(&mut classes);

  classes
}
