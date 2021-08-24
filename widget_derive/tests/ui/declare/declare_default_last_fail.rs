use ribir::prelude::*;

fn widget() -> BoxedWidget {
  declare! {
    BoxDecoration {
      ..<_>::default(),
      background: Some(Color::RED.into()),
    }
  }
}

fn main() {}
