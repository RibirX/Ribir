use ribir::prelude::*;
use ribir::prelude::{include_crate_svg, Svg};
use ribir_dev_helper::*;

#[test]
fn include_svg() {
  let svg: Svg = include_crate_svg!("./assets/test1.svg");
  assert_eq!(svg.paint_commands.len(), 2);
}

fn fix_draw_svg_not_apply_alpha() -> Painter {
  let mut painter = Painter::new(Rect::from_size(Size::new(64., 64.)));
  let svg: Svg = include_crate_svg!("./assets/test1.svg");
  painter.apply_alpha(0.5).draw_svg(&svg);
  painter
}

painter_backend_eq_image_test!(fix_draw_svg_not_apply_alpha);
