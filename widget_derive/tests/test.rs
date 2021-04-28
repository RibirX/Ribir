use holiday::prelude::*;

#[derive(Widget, Debug)]
struct A;

#[test]
fn derive_widget() {
  let a = A;
  a.with_key("a").box_it();
}

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/*fail.rs");
  t.pass("tests/ui/*pass.rs");
}
