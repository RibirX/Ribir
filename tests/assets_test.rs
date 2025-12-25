use ribir::prelude::{asset, include_asset, *};
use ribir_dev_helper::*;

#[test]
fn include_svg() {
  let svg: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true, inherit_stroke = false);
  // Command size may vary between compile-time embedding and runtime loading
  assert!(svg.command_size() > 0);
}

fn fix_draw_svg_not_apply_alpha() -> Painter {
  let mut painter = Painter::new(Rect::from_size(Size::new(64., 64.)));
  let svg: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true, inherit_stroke = false);
  painter.apply_alpha(0.5).draw_svg(&svg);
  painter
}

painter_backend_eq_image_test!(fix_draw_svg_not_apply_alpha, comparison = 0.004);

// Asset macro tests

#[test]
fn asset_svg_basic() {
  let svg: Svg = asset!("./assets/test1.svg", "svg");
  // Just verify it loads successfully
  assert!(svg.command_size() > 0);
}

#[test]
fn asset_svg_case_insensitive() {
  // Test case-insensitive type matching
  let svg1: Svg = asset!("./assets/test1.svg", "svg");
  let svg2: Svg = asset!("./assets/test1.svg", "SVG");
  let svg3: Svg = asset!("./assets/test1.svg", "Svg");

  assert_eq!(svg1.command_size(), svg2.command_size());
  assert_eq!(svg2.command_size(), svg3.command_size());
}

#[test]
fn asset_svg_with_inherit_fill() {
  // Test SVG with inherit_fill parameter
  let svg: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true);
  // Verify it loads successfully (command size may differ with inherit_fill)
  assert!(svg.command_size() > 0);
}

#[test]
fn asset_svg_with_both_parameters() {
  // Test SVG with both inherit_fill and inherit_stroke parameters
  let svg: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true, inherit_stroke = false);
  // Verify it loads successfully
  assert!(svg.command_size() > 0);
}

#[test]
fn asset_svg_gradient() {
  // Test with a different SVG file
  let svg: Svg = asset!("./assets/fill_with_gradient.svg", "svg");
  // Just verify it loads successfully
  assert!(svg.command_size() > 0);
}

#[test]
fn asset_svg_all_parameters() {
  // Test all parameter combinations work correctly
  let svg1: Svg = asset!("./assets/test1.svg", "svg");
  let svg2: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true);
  let svg3: Svg = asset!("./assets/test1.svg", "svg", inherit_stroke = true);
  let svg4: Svg = asset!("./assets/test1.svg", "svg", inherit_fill = true, inherit_stroke = true);

  // All should load successfully (command counts may vary based on parameters)
  assert!(svg1.command_size() > 0);
  assert!(svg2.command_size() > 0);
  assert!(svg3.command_size() > 0);
  assert!(svg4.command_size() > 0);
}

#[test]
fn asset_conflict_resolution() {
  // Test that files with the same name from different directories don't conflict
  let asset1: Vec<u8> = asset!("./assets_conflict/dir1/test.txt");
  let asset2: Vec<u8> = asset!("./assets_conflict/dir2/test.txt");

  // Verify both assets loaded successfully
  assert!(!asset1.is_empty(), "First asset should not be empty");
  assert!(!asset2.is_empty(), "Second asset should not be empty");

  // Verify they contain different content (proving they're different files)
  let content1 = String::from_utf8(asset1).unwrap();
  let content2 = String::from_utf8(asset2).unwrap();

  assert_ne!(
    content1, content2,
    "Files with same name from different directories should have different content"
  );
  assert!(content1.contains("dir1"), "First asset should contain 'dir1'");
  assert!(content2.contains("dir2"), "Second asset should contain 'dir2'");

  println!("✅ Asset conflict resolution test passed!");
  println!("Asset 1: {}", content1.trim());
  println!("Asset 2: {}", content2.trim());
}

// Include Asset macro tests

#[test]
fn include_asset_svg_basic() {
  let svg: Svg = include_asset!("./assets/test1.svg", "svg");
  assert!(svg.command_size() > 0);
}

#[test]
fn include_asset_text() {
  // reusing one of the text files from conflict test
  let content: String = include_asset!("./assets_conflict/dir1/test.txt", "text");
  assert!(content.contains("dir1"));
}

#[test]
fn include_asset_binary() {
  // reusing one of the text files from conflict test as binary
  let data: Vec<u8> = include_asset!("./assets_conflict/dir1/test.txt");
  let content = String::from_utf8(data).unwrap();
  assert!(content.contains("dir1"));
}

// Image Asset macro tests

#[test]
fn asset_image_basic() {
  use ribir_core::prelude::Image;
  let img: Image = asset!("./assets/test_image.png", "image");
  // Verify it loads successfully and has correct dimensions
  assert!(img.width() > 0);
  assert!(img.height() > 0);
  assert_eq!(img.frame_count(), 1);
  assert!(!img.is_animated());
}

#[test]
fn asset_image_first_frame() {
  use ribir_core::prelude::Image;
  let img: Image = asset!("./assets/test_image.png", "image");
  let frame = img.first_frame();
  // Verify the frame can be decoded
  assert!(frame.image.width() > 0);
  assert!(frame.image.height() > 0);
}

#[test]
fn include_asset_image() {
  use ribir_core::prelude::Image;
  let img: Image = include_asset!("./assets/test_image.png", "image");
  assert!(img.width() > 0);
  assert!(!img.is_animated());
}

#[test]
fn asset_animated_gif() {
  use ribir_core::prelude::Image;
  let img: Image = asset!("./assets/test_animated.gif", "image");
  // Verify animated GIF is converted to animated WebP
  assert_eq!(img.width(), 32);
  assert_eq!(img.height(), 32);
  assert!(img.frame_count() > 1, "Expected animated image with multiple frames");
  assert!(img.is_animated());
}

#[test]
fn include_asset_animated_gif() {
  use ribir_core::prelude::Image;
  let img: Image = include_asset!("./assets/test_animated.gif", "image");
  assert_eq!(img.width(), 32);
  assert!(img.is_animated());
  assert!(img.frame_count() > 1);
}
