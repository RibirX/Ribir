use holiday::prelude::*;

#[derive(Widget)]
struct Test {
  field: f32,
}

#[test]
fn derive_widget() {
  let a = Test { field: 1. };
  a.with_key("a").box_it();
}

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/*fail.rs");
  t.pass("tests/ui/*pass.rs");
}
