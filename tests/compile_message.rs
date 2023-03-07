use ribir::prelude::{include_svg, SvgPaths};

#[test]
fn compile_msg() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/compile_msg/**/*fail.rs");
  t.pass("tests/compile_msg/**/*pass.rs");
}

#[test]
fn include_svg() {
  let svg: SvgPaths = include_svg!("./assets/test1.svg");
  assert_eq!(svg.paths.len(), 2);
}
