use ribir::prelude::{include_svg, Svg};

#[test]
fn include_svg() {
  let svg: Svg = include_svg!("./assets/test1.svg");
  assert_eq!(svg.paths.len(), 2);
}
