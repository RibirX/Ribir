#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/**/*fail.rs");
  t.pass("tests/ui/**/*pass.rs");
}

use ribir::prelude::*;
#[test]
fn embed_widget_ref_outside() {
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      let size = Size::zero();
      widget! {
        declare Flex {
          SizedBox { size }
          declare SizedBox { size }
        }
      }
    }
  }
}
