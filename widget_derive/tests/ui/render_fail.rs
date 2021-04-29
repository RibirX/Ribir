use holiday::prelude::*;

#[derive(Widget, RenderWidget)]
struct A;

#[derive(Widget, RenderWidget)]
struct B {
  #[proxy]
  a: Flex,
  #[proxy]
  b: Flex,
}

fn main() { let a = A; }
