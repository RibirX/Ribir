use ribir::prelude::*;

#[derive(CombinationWidget)]
struct A;

#[derive(CombinationWidget)]
struct B {
  #[proxy]
  a: Checkbox,
  #[proxy]
  b: Checkbox,
}

fn main() { let a = A; }
