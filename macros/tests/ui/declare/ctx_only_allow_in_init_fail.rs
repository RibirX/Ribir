use ribir::prelude::*;

fn main() {
  let _ctx_use_in_init_pass = widget! {
    init ctx => {
      let primary = Palette::of(ctx).primary();
    }
    SizedBox { size: ZERO_SIZE, background: primary }
  };

  let _ctx_not_allow_in_other_phase = widget! {
    init ctx => {}
    SizedBox {
      size: ZERO_SIZE, background: Palette::of(ctx).primary()
    }
  };
}
