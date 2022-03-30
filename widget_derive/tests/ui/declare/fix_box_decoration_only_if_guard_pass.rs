use ribir::prelude::*;

struct T;
impl CombinationWidget for T {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare SizedBox {
        size: Size::zero(),
        background if true => : Color::RED,
      }
    }
  }
}

fn main() {}
