use ribir::prelude::*;

fn main() {
  compile_error!("Test for declare syntax warning.");
}

struct NeedlessDeclareWarning;
impl CombinationWidget for NeedlessDeclareWarning {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    let size = Size::zero();
    widget! {
      declare SizedBox {
        size,
        declare SizedBox { size }
      }
    }
  }
}
