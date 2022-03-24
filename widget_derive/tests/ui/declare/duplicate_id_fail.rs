use ribir::prelude::*;

fn id_must_be_unique_err(ctx: &mut BuildCtx) {
  declare! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      ..<_>::default(),
      SizedBox {
        id: same_id,
        size: Size::zero(),
      }
    }
  };
}

fn main() {}
