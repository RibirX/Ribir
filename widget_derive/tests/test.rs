use widget_derive::include_svg;

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/**/*fail.rs");
  t.pass("tests/ui/**/*pass.rs");
}

#[test]
fn include_svg() {
  use painter::SvgRender;
  let svg: SvgRender = include_svg!("./assets/test1.svg");
  assert_eq!(svg.paths.len(), 2);
}
