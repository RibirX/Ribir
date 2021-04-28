use holiday::prelude::*;

#[derive(Widget, RenderWidget, Debug)]
struct A;

#[derive(Debug, Widget, RenderWidget)]
struct B {
  #[proxy]
  a: Flex,
  #[proxy]
  b: Flex,
}

fn main() { let a = A; }
