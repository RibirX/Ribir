use ribir_core::prelude::Classes;

mod scrollbar_cls;

pub fn initd_classes() -> Classes {
  let mut classes = Classes::default();
  scrollbar_cls::init(&mut classes);
  classes
}
