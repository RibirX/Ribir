use holiday::prelude::*;


#[derive(Debug, Widget, RenderWidget)]
struct B<W: std::fmt::Debug + 'static> {
  #[proxy]
  a: W,
}

fn main() {
  let b = B { a: SizedBox::empty_box(Size::zero()) };
  let _: Box<dyn RenderWidgetSafety> = Box::new(b);
}
