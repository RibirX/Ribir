use ribir_core::prelude::Classes;

mod icon_cls;

pub fn initd_classes() -> Classes {
  let mut classes = Classes::default();

  icon_cls::init(&mut classes);
  classes
}
