use ribir::prelude::{include_crate_svg, *};
use ribir_dev_helper::*;

#[test]
fn include_svg() {
  let svg: Svg = include_crate_svg!("./assets/test1.svg", true, false);
  assert_eq!(svg.command_size(), 2);
}

fn fix_draw_svg_not_apply_alpha() -> Painter {
  let mut painter = Painter::new(Rect::from_size(Size::new(64., 64.)));
  let svg: Svg = include_crate_svg!("./assets/test1.svg", true, false);
  painter.apply_alpha(0.5).draw_svg(&svg);
  painter
}

painter_backend_eq_image_test!(fix_draw_svg_not_apply_alpha, comparison = 0.002);
