use holiday::prelude::*;


#[derive(Debug, Widget, RenderWidget)]
struct B<W: std::fmt::Debug + 'static> {
  #[proxy]
  a: W,
}

// Support tuple struct.
#[derive(Debug, Widget, RenderWidget)]
struct C(#[proxy] SizedBox);

fn main() {
  let b = B { a: SizedBox::empty_box(Size::zero()) };
  let _: Box<dyn RenderWidgetSafety> = Box::new(b);

  let c = C(SizedBox::empty_box(Size::zero()));
  let _: Box<dyn RenderWidgetSafety> = Box::new(c);
}
