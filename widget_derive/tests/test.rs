use holiday::prelude::*;

#[derive(Widget, Debug)]
struct A;

#[test]
fn derive_widget() {
  let a = A;
  a.with_key("a").box_it();
}
