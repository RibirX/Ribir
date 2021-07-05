use holiday::prelude::*;

#[derive(Widget, CombinationWidget)]
struct A;

#[derive(Widget, CombinationWidget)]
struct B {
  #[proxy]
  a: Checkbox,
  #[proxy]
  b: Checkbox,
}

fn main() { let a = A; }
