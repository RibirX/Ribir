use ribir::prelude::*;

#[derive(RenderWidget)]
struct A;

#[derive(RenderWidget)]
struct B {
  #[proxy]
  a: Flex,
  #[proxy]
  b: Flex,
}

fn main() { let a = A; }
