use ribir::prelude::*;

#[derive(RenderWidget)]
struct B<W> {
  #[proxy]
  a: W,
}

// Support tuple struct.
#[derive(RenderWidget)]
struct C(#[proxy] SizedBox);

fn main() {
  let b = B { a: SizedBox::from_size(Size::zero()) };
  let _: Box<dyn RenderWidgetSafety> = Box::new(b);

  let c = C(SizedBox::from_size(Size::zero()));
  let _: Box<dyn RenderWidgetSafety> = Box::new(c);
}
