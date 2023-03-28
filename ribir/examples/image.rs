use ribir::prelude::*;

fn main() {
  let img = ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"));
  let w = widget! {
    Column {
      SizedBox {
        size: Size::new(100., 100.),
        DynWidget::from(img.clone())
      }
      DynWidget::from(img)
    }
  };

  app::run(w);
}
