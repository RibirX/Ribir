use holiday::prelude::*;

#[derive(Widget, CombinationWidget, Debug)]
struct A;

#[derive(Debug, Widget, CombinationWidget)]
struct B {
  #[proxy]
  a: Checkbox,
  #[proxy]
  b: Checkbox,
}

fn main() { let a = A; }
