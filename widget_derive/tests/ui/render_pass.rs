use holiday::prelude::*;

#[derive(Widget, RenderWidget)]
struct B<W> {
  #[proxy]
  a: W,
}

// Support tuple struct.
#[derive(Widget, RenderWidget)]
struct C(#[proxy] SizedBox);

fn main() {
  let b = B { a: SizedBox::empty_box(Size::zero()) };
  let _: Box<dyn RenderWidgetSafety> = Box::new(b);

  let c = C(SizedBox::empty_box(Size::zero()));
  let _: Box<dyn RenderWidgetSafety> = Box::new(c);
}
