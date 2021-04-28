use holiday::prelude::*;

#[derive(Debug, Widget)]
struct A;

impl CombinationWidget for A {
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    unimplemented!();
  }
}

#[derive(Debug, Widget, CombinationWidget)]
struct B<W: std::fmt::Debug + 'static> {
  #[proxy]
  a: W,
}

fn main() {
  let b = B { a: A };
  let _: Box<dyn CombinationWidget> = Box::new(b);
}