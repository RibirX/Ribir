#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/**/*fail.rs");
  t.pass("tests/ui/**/*pass.rs");
}

use ribir::prelude::*;
struct T;
impl Compose for T {
  #[widget]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare SizedBox {
        size: Size::zero(),
        background if true => : Color::RED,
      }
    }
  }
}
