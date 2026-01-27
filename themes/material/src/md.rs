//! This module defines the constant variables of the material theme that are
//! frequently used.
//!
//! See: https://m3.material.io

use ribir_core::prelude::*;

// The motion variables
// See https://m3.material.io/styles/motion/overview
pub mod easing {
  use ribir_core::animation::easing::CubicBezierEasing;
  pub use ribir_core::animation::easing::LINEAR;

  // The Emphasized easing set is recommended for most transitions to capture the
  // style of M3.
  pub const EMPHASIZED: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
  pub const EMPHASIZED_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 0.8, 0.15);
  pub const EMPHASIZED_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0.05, 0.7, 0.1, 1.);

  // The Standard easing set can be used for small utility focused transitions
  // that need to be quick.
  pub const STANDARD: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
  pub const STANDARD_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 1., 1.);
  pub const STANDARD_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0., 0., 0., 1.);

  pub mod duration {

    use crate::Duration;
    // ## Short duration
    // These are used for small utility-focused transitions.
    pub const SHORT1: Duration = Duration::from_millis(50);
    pub const SHORT2: Duration = Duration::from_millis(100);
    pub const SHORT3: Duration = Duration::from_millis(150);
    pub const SHORT4: Duration = Duration::from_millis(200);

    // ## Medium durations
    // These are used for transitions that traverse a medium area of the screen.
    pub const MEDIUM1: Duration = Duration::from_millis(250);
    pub const MEDIUM2: Duration = Duration::from_millis(300);
    pub const MEDIUM3: Duration = Duration::from_millis(350);
    pub const MEDIUM4: Duration = Duration::from_millis(400);

    // ## Long durations
    // These durations are often paired with Emphasized easing. They're used for
    // large expressive transitions.
    pub const LONG1: Duration = Duration::from_millis(450);
    pub const LONG2: Duration = Duration::from_millis(500);
    pub const LONG3: Duration = Duration::from_millis(550);
    pub const LONG4: Duration = Duration::from_millis(600);

    // ## Extra long durations
    // Though rare, some transitions use durations above 600ms. These are usually
    // used for ambient transitions that don't involve user input.
    pub const EXR_LONG1: Duration = Duration::from_millis(700);
    pub const EXR_LONG2: Duration = Duration::from_millis(800);
    pub const EXR_LONG3: Duration = Duration::from_millis(900);
    pub const EXR_LONG4: Duration = Duration::from_millis(1000);
  }
}

// These are the constant variables commonly used in the material theme.

pub const RADIUS_5: Radius = Radius::all(5.);
pub const RADIUS_10: Radius = Radius::all(10.);
pub const RADIUS_12: Radius = Radius::all(12.);
pub const RADIUS_16: Radius = Radius::all(16.);
pub const RADIUS_20: Radius = Radius::all(20.);
pub const RADIUS_28: Radius = Radius::all(28.);

pub const RADIUS_1: Radius = Radius::all(1.);
pub const RADIUS_2: Radius = Radius::all(2.);
pub const RADIUS_4: Radius = Radius::all(4.);
pub const RADIUS_8: Radius = Radius::all(8.);

pub const THICKNESS_4: f32 = 4.;
pub const THICKNESS_8: f32 = 8.;

pub const SIZE_10: Size = Size::new(10., 10.);
pub const SIZE_20: Size = Size::new(20., 20.);
pub const SIZE_40: Size = Size::new(40., 40.);

pub const SIZE_4: Size = Size::new(4., 4.);
pub const SIZE_8: Size = Size::new(8., 8.);
pub const SIZE_18: Size = Size::new(18., 18.);
pub const SIZE_24: Size = Size::new(24., 24.);
pub const SIZE_36: Size = Size::new(36., 36.);
pub const SIZE_48: Size = Size::new(48., 48.);
pub const SIZE_64: Size = Size::new(64., 64.);

pub const EDGES_2: EdgeInsets = EdgeInsets::all(2.);
pub const EDGES_HOR_2: EdgeInsets = EdgeInsets::horizontal(2.);
pub const EDGES_VER_2: EdgeInsets = EdgeInsets::vertical(2.);

pub const EDGES_4: EdgeInsets = EdgeInsets::all(4.);
pub const EDGES_HOR_4: EdgeInsets = EdgeInsets::horizontal(4.);
pub const EDGES_VER_4: EdgeInsets = EdgeInsets::vertical(4.);
pub const EDGES_LEFT_4: EdgeInsets = EdgeInsets::only_left(4.);
pub const EDGES_RIGHT_4: EdgeInsets = EdgeInsets::only_right(4.);
pub const EDGES_TOP_4: EdgeInsets = EdgeInsets::only_top(4.);
pub const EDGES_BOTTOM_4: EdgeInsets = EdgeInsets::only_bottom(4.);

pub const EDGES_HOR_6: EdgeInsets = EdgeInsets::horizontal(6.);

pub const EDGES_8: EdgeInsets = EdgeInsets::all(8.);
pub const EDGES_HOR_8: EdgeInsets = EdgeInsets::horizontal(8.);
pub const EDGES_VER_8: EdgeInsets = EdgeInsets::vertical(8.);
pub const EDGES_LEFT_8: EdgeInsets = EdgeInsets::only_left(8.);
pub const EDGES_RIGHT_8: EdgeInsets = EdgeInsets::only_right(8.);
pub const EDGES_TOP_8: EdgeInsets = EdgeInsets::only_top(8.);
pub const EDGES_BOTTOM_8: EdgeInsets = EdgeInsets::only_bottom(8.);

pub const EDGES_10: EdgeInsets = EdgeInsets::all(10.);
pub const EDGES_VER_10: EdgeInsets = EdgeInsets::vertical(10.);
pub const EDGES_TOP_10: EdgeInsets = EdgeInsets::only_top(10.);
pub const EDGES_BOTTOM_10: EdgeInsets = EdgeInsets::only_bottom(10.);

pub const EDGES_HOR_12: EdgeInsets = EdgeInsets::horizontal(12.);
pub const EDGES_VER_12: EdgeInsets = EdgeInsets::vertical(12.);

pub const EDGES_VER_14: EdgeInsets = EdgeInsets::vertical(14.);

pub const EDGES_16: EdgeInsets = EdgeInsets::all(16.);
pub const EDGES_HOR_16: EdgeInsets = EdgeInsets::horizontal(16.);
pub const EDGES_VER_16: EdgeInsets = EdgeInsets::vertical(16.);
pub const EDGES_LEFT_16: EdgeInsets = EdgeInsets::only_left(16.);
pub const EDGES_RIGHT_16: EdgeInsets = EdgeInsets::only_right(16.);
pub const EDGES_TOP_16: EdgeInsets = EdgeInsets::only_top(16.);
pub const EDGES_BOTTOM_16: EdgeInsets = EdgeInsets::only_bottom(16.);

pub const EDGES_LEFT_20: EdgeInsets = EdgeInsets::only_left(20.);

pub const EDGES_HOR_24: EdgeInsets = EdgeInsets::horizontal(24.);
pub const EDGES_LEFT_24: EdgeInsets = EdgeInsets::only_left(24.);

pub const EDGES_HOR_32: EdgeInsets = EdgeInsets::horizontal(32.);
pub const EDGES_HOR_36: EdgeInsets = EdgeInsets::horizontal(36.);
pub const EDGES_HOR_48: EdgeInsets = EdgeInsets::horizontal(48.);

// Borders
pub fn border_1(color: Color) -> Border { Border::all(BorderSide::new(1., color.into())) }

pub fn border_1_top(color: Color) -> Border { Border::only_top(BorderSide::new(1., color.into())) }

pub fn border_1_right(color: Color) -> Border {
  Border::only_right(BorderSide::new(1., color.into()))
}
pub fn border_1_bottom(color: Color) -> Border {
  Border::only_bottom(BorderSide::new(1., color.into()))
}
pub fn border_1_left(color: Color) -> Border {
  Border::only_left(BorderSide::new(1., color.into()))
}

pub fn border_2() -> VariantMap<Variant<Color>, impl Fn(&Color) -> Border> {
  BuildCtx::color().map(|color| Border::all(BorderSide::new(2., (*color).into())))
}
pub fn border_2_surface_color() -> Border {
  let surface_variant = Palette::of(BuildCtx::get()).on_surface_variant();
  Border::all(BorderSide::new(2., surface_variant.into()))
}

/// M3 elevation shadow for a given level (0-5).
///
/// See: https://m3.material.io/styles/elevation/overview
///
/// - Level 0: 0dp (no shadow)
/// - Level 1: 1dp
/// - Level 2: 3dp
/// - Level 3: 6dp (FAB resting)
/// - Level 4: 8dp (FAB hover)
/// - Level 5: 12dp
pub fn elevation_shadow(level: u8, shadow_color: Color) -> BoxShadow {
  // M3 elevation dp values and alpha (higher elevation = slightly more visible
  // shadow)
  let (offset_y, blur, spread, alpha) = match level {
    0 => (0., 0., 0., 0.),
    1 => (1., 2., 0., 0.10),
    2 => (2., 4., 0., 0.12),
    3 => (3., 6., 1., 0.15),
    4 => (4., 8., 1., 0.18),
    _ => (6., 12., 2., 0.22), // Level 5+
  };
  BoxShadow::new(Point::new(0., offset_y), blur, spread, shadow_color.with_alpha(alpha as f32))
}
