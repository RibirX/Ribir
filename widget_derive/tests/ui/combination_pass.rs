use ribir::prelude::*;

#[derive(Debug)]
struct A;

impl CombinationWidget for A {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    unreachable!();
  }
}

#[derive(Debug, CombinationWidget)]
struct B<W> {
  #[proxy]
  a: W,
}

#[derive(Debug, CombinationWidget)]
struct TupleB<W>(#[proxy] W);

fn main() {
  let b = B { a: A };
  let _: Box<dyn CombinationWidget> = Box::new(b);
}
