use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Icon {
  pub src: &'static str,
  pub size: Size,
}

impl Compose for Icon {
  fn compose(self, _: &mut BuildCtx) -> BoxedWidget {
    let svg = Svg::new(load_src(self.src).unwrap());
    widget! {
      declare SizedBox {
        size: self.size,
        ExprChild { svg }
      }
    }
  }
}
