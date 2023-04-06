use ribir::prelude::*;

fn main() {
  let img = ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"));
  let img2 = ShallowImage::from_png(include_bytes!(
    "../../ribir/examples/attachments/3DDD-1.png"
  ));
  let w = widget! {
    Column {
      SizedBox {
        size: Size::new(100., 100.),
        DynWidget::from(img)
      }
      DynWidget::from(img2)
    }
  };

  app::run(w);
}
