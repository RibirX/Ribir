use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Icon {
  pub src: &'static str,
  pub size: Size,
}

impl Compose for Icon {
  fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
    let svg = Svg::new(load_src(this.state_ref().src).unwrap());
    widget! {
      track { this }
      SizedBox {
        size: this.size,
        ExprWidget { expr: svg }
      }
    }
  }
}
