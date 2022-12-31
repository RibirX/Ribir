use ribir_macros::include_svg;

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/**/*fail.rs");
  t.pass("tests/ui/**/*pass.rs");
}

#[test]
fn include_svg() {
  use ribir_painter::SvgPaths;
  let svg: SvgPaths = include_svg!("./assets/test1.svg");
  assert_eq!(svg.paths.len(), 2);
}
