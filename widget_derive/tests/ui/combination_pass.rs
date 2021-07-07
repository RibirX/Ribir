use ribir::prelude::*;

#[derive(Debug, Widget)]
struct A;

impl CombinationWidget for A {
  fn build(&self, _: &mut BuildCtx) -> Box<dyn Widget> {
    unimplemented!();
  }
}

#[derive(Debug, Widget, CombinationWidget)]
struct B<W> {
  #[proxy]
  a: W,
}

#[derive(Debug, Widget, CombinationWidget)]
struct TupleB<W>(#[proxy] W);

fn main() {
  let b = B { a: A };
  let _: Box<dyn CombinationWidget> = Box::new(b);
}
