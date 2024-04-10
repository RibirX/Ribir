use material_color_utilities_rs::htc;
use ribir_geom::Point;
use serde::{Deserialize, Serialize};

use crate::SpreadMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Color {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
  pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GradientStop {
  pub color: Color,
  pub offset: f32,
}

impl GradientStop {
  #[inline]
  pub fn new(color: Color, offset: f32) -> Self { Self { color, offset } }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RadialGradient {
  pub start_center: Point,
  pub start_radius: f32,
  pub end_center: Point,
  pub end_radius: f32,
  pub stops: Vec<GradientStop>,
  pub spread_method: SpreadMethod,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LinearGradient {
  pub start: Point,
  pub end: Point,
  pub stops: Vec<GradientStop>,
  pub spread_method: SpreadMethod,
}

/// Describe the light tone of a color, should between [0, 1.0], 0.0 gives
/// absolute black and 1.0 give the brightest white.
#[derive(Clone, Debug, Copy)]
pub struct LightnessTone(f32);

impl LightnessTone {
  #[inline]
  pub fn new(tone: f32) -> Self { Self(tone.clamp(0., 1.0)) }
}

impl Color {
  #[inline]
  pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
    Self { red, green, blue, alpha }
  }

  #[inline]
  pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self { Self::new(r, g, b, 255) }

  #[inline]
  pub fn from_f32_rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
    Self {
      red: f32_component_to_u8(red),
      green: f32_component_to_u8(green),
      blue: f32_component_to_u8(blue),
      alpha: f32_component_to_u8(alpha),
    }
  }

  #[inline]
  pub fn from_u32(rgba: u32) -> Self {
    let bytes = rgba.to_be_bytes();
    Self { red: bytes[0], green: bytes[1], blue: bytes[2], alpha: bytes[3] }
  }

  #[inline]
  pub fn into_u32(self) -> u32 {
    let Self { red, green, blue, alpha } = self;
    u32::from_be_bytes([red, green, blue, alpha])
  }

  #[inline]
  pub fn with_alpha(mut self, alpha: f32) -> Self {
    self.alpha = f32_component_to_u8(alpha);
    self
  }

  /// return an new color after the color applied alpha.
  #[inline]
  pub fn apply_alpha(mut self, alpha: f32) -> Self {
    let base: f32 = u8_component_to_f32(self.alpha);
    self.alpha = f32_component_to_u8(base * alpha);
    self
  }

  #[inline]
  pub fn with_lightness(self, l: LightnessTone) -> Self {
    let mut hct = htc::Hct::from_int([self.alpha, self.red, self.green, self.blue]);
    hct.set_tone((l.0 * 100.).clamp(0., 100.) as f64);
    let argb = hct.to_int();
    Self { red: argb[1], green: argb[2], blue: argb[3], alpha: argb[0] }
  }

  #[inline]
  pub fn into_components(self) -> [u8; 4] {
    let Self { red, green, blue, alpha } = self;
    [red, green, blue, alpha]
  }

  #[inline]
  pub fn into_f32_components(self) -> [f32; 4] {
    let Self { red, green, blue, alpha } = self;
    [
      u8_component_to_f32(red),
      u8_component_to_f32(green),
      u8_component_to_f32(blue),
      u8_component_to_f32(alpha),
    ]
  }
}

const C23: u32 = 0x4b00_0000;
// Algorithm from https://github.com/Ogeon/palette/pull/184/files.
fn u8_component_to_f32(v: u8) -> f32 {
  let comp_u = v as u32 + C23;
  let comp_f = f32::from_bits(comp_u) - f32::from_bits(C23);
  let max_u = u8::MAX as u32 + C23;
  let max_f = (f32::from_bits(max_u) - f32::from_bits(C23)).recip();
  comp_f * max_f
}

// Algorithm from https://github.com/Ogeon/palette/pull/184/files.
fn f32_component_to_u8(v: f32) -> u8 {
  let max = u8::MAX as f32;
  let scaled = (v * max).min(max);
  let f = scaled + f32::from_bits(C23);
  (f.to_bits().saturating_sub(C23)) as u8
}

impl Color {
  // from css3 keywords: https://www.w3.org/wiki/CSS/Properties/color/keywords
  pub const ALICEBLUE: Color = Self::from_rgb(240, 248, 255);
  pub const ANTIQUEWHITE: Color = Self::from_rgb(250, 235, 215);
  pub const AQUA: Color = Self::from_rgb(0, 255, 255);
  pub const AQUAMARINE: Color = Self::from_rgb(127, 255, 212);

  pub const AZURE: Color = Self::from_rgb(240, 255, 255);

  pub const BEIGE: Color = Self::from_rgb(245, 245, 220);
  pub const BISQUE: Color = Self::from_rgb(255, 228, 196);
  pub const BLACK: Color = Self::from_rgb(0, 0, 0);
  pub const BLANCHEDALMOND: Color = Self::from_rgb(255, 235, 205);

  pub const BLUE: Color = Self::from_rgb(0, 0, 255);

  pub const BLUEVIOLET: Color = Self::from_rgb(138, 43, 226);
  pub const BROWN: Color = Self::from_rgb(165, 42, 42);
  pub const BURLYWOOD: Color = Self::from_rgb(222, 184, 135);
  pub const CADETBLUE: Color = Self::from_rgb(95, 158, 160);

  pub const CHARTREUSE: Color = Self::from_rgb(127, 255, 0);

  pub const CHOCOLATE: Color = Self::from_rgb(210, 105, 30);
  pub const CORAL: Color = Self::from_rgb(255, 127, 80);
  pub const CORNFLOWERBLUE: Color = Self::from_rgb(100, 149, 237);
  pub const CORNSILK: Color = Self::from_rgb(255, 248, 220);

  pub const CRIMSON: Color = Self::from_rgb(220, 20, 60);

  pub const CYAN: Color = Self::from_rgb(0, 255, 255);
  pub const DARKBLUE: Color = Self::from_rgb(0, 0, 139);
  pub const DARKCYAN: Color = Self::from_rgb(0, 139, 139);
  pub const DARKGOLDENROD: Color = Self::from_rgb(184, 134, 11);

  pub const DARKGRAY: Color = Self::from_rgb(169, 169, 169);

  pub const DARKGREEN: Color = Self::from_rgb(0, 100, 0);
  pub const DARKGREY: Color = Self::from_rgb(169, 169, 169);
  pub const DARKKHAKI: Color = Self::from_rgb(189, 183, 107);
  pub const DARKMAGENTA: Color = Self::from_rgb(139, 0, 139);

  pub const DARKOLIVEGREEN: Color = Self::from_rgb(85, 107, 47);

  pub const DARKORANGE: Color = Self::from_rgb(255, 140, 0);
  pub const DARKORCHID: Color = Self::from_rgb(153, 50, 204);
  pub const DARKRED: Color = Self::from_rgb(139, 0, 0);
  pub const DARKSALMON: Color = Self::from_rgb(233, 150, 122);

  pub const DARKSEAGREEN: Color = Self::from_rgb(143, 188, 143);

  pub const DARKSLATEBLUE: Color = Self::from_rgb(72, 61, 139);
  pub const DARKSLATEGRAY: Color = Self::from_rgb(47, 79, 79);
  pub const DARKSLATEGREY: Color = Self::from_rgb(47, 79, 79);
  pub const DARKTURQUOISE: Color = Self::from_rgb(0, 206, 209);

  pub const DARKVIOLET: Color = Self::from_rgb(148, 0, 211);

  pub const DEEPPINK: Color = Self::from_rgb(255, 20, 147);
  pub const DEEPSKYBLUE: Color = Self::from_rgb(0, 191, 255);
  pub const DIMGRAY: Color = Self::from_rgb(105, 105, 105);
  pub const DIMGREY: Color = Self::from_rgb(105, 105, 105);

  pub const DODGERBLUE: Color = Self::from_rgb(30, 144, 255);

  pub const FIREBRICK: Color = Self::from_rgb(178, 34, 34);
  pub const FLORALWHITE: Color = Self::from_rgb(255, 250, 240);
  pub const FORESTGREEN: Color = Self::from_rgb(34, 139, 34);
  pub const FUCHSIA: Color = Self::from_rgb(255, 0, 255);

  pub const GAINSBORO: Color = Self::from_rgb(220, 220, 220);

  pub const GHOSTWHITE: Color = Self::from_rgb(248, 248, 255);
  pub const GOLD: Color = Self::from_rgb(255, 215, 0);
  pub const GOLDENROD: Color = Self::from_rgb(218, 165, 32);
  pub const GRAY: Color = Self::from_rgb(128, 128, 128);

  pub const GREEN: Color = Self::from_rgb(0, 128, 0);

  pub const GREENYELLOW: Color = Self::from_rgb(173, 255, 47);
  pub const GREY: Color = Self::from_rgb(128, 128, 128);
  pub const HONEYDEW: Color = Self::from_rgb(240, 255, 240);
  pub const HOTPINK: Color = Self::from_rgb(255, 105, 180);

  pub const INDIANRED: Color = Self::from_rgb(205, 92, 92);

  pub const INDIGO: Color = Self::from_rgb(75, 0, 130);
  pub const IVORY: Color = Self::from_rgb(255, 255, 240);
  pub const KHAKI: Color = Self::from_rgb(240, 230, 140);
  pub const LAVENDER: Color = Self::from_rgb(230, 230, 250);

  pub const LAVENDERBLUSH: Color = Self::from_rgb(255, 240, 245);

  pub const LAWNGREEN: Color = Self::from_rgb(124, 252, 0);
  pub const LEMONCHIFFON: Color = Self::from_rgb(255, 250, 205);
  pub const LIGHTBLUE: Color = Self::from_rgb(173, 216, 230);
  pub const LIGHTCORAL: Color = Self::from_rgb(240, 128, 128);

  pub const LIGHTCYAN: Color = Self::from_rgb(224, 255, 255);

  pub const LIGHTGOLDENRODYELLOW: Color = Self::from_rgb(250, 250, 210);
  pub const LIGHTGRAY: Color = Self::from_rgb(211, 211, 211);
  pub const LIGHTGREEN: Color = Self::from_rgb(144, 238, 144);
  pub const LIGHTGREY: Color = Self::from_rgb(211, 211, 211);

  pub const LIGHTPINK: Color = Self::from_rgb(255, 182, 193);

  pub const LIGHTSALMON: Color = Self::from_rgb(255, 160, 122);
  pub const LIGHTSEAGREEN: Color = Self::from_rgb(32, 178, 170);
  pub const LIGHTSKYBLUE: Color = Self::from_rgb(135, 206, 250);
  pub const LIGHTSLATEGRAY: Color = Self::from_rgb(119, 136, 153);

  pub const LIGHTSLATEGREY: Color = Self::from_rgb(119, 136, 153);

  pub const LIGHTSTEELBLUE: Color = Self::from_rgb(176, 196, 222);
  pub const LIGHTYELLOW: Color = Self::from_rgb(255, 255, 224);
  pub const LIME: Color = Self::from_rgb(0, 255, 0);
  pub const LIMEGREEN: Color = Self::from_rgb(50, 205, 50);

  pub const LINEN: Color = Self::from_rgb(250, 240, 230);

  pub const MAGENTA: Color = Self::from_rgb(255, 0, 255);
  pub const MAROON: Color = Self::from_rgb(128, 0, 0);
  pub const MEDIUMAQUAMARINE: Color = Self::from_rgb(102, 205, 170);
  pub const MEDIUMBLUE: Color = Self::from_rgb(0, 0, 205);

  pub const MEDIUMORCHID: Color = Self::from_rgb(186, 85, 211);

  pub const MEDIUMPURPLE: Color = Self::from_rgb(147, 112, 219);
  pub const MEDIUMSEAGREEN: Color = Self::from_rgb(60, 179, 113);
  pub const MEDIUMSLATEBLUE: Color = Self::from_rgb(123, 104, 238);
  pub const MEDIUMSPRINGGREEN: Color = Self::from_rgb(0, 250, 154);

  pub const MEDIUMTURQUOISE: Color = Self::from_rgb(72, 209, 204);

  pub const MEDIUMVIOLETRED: Color = Self::from_rgb(199, 21, 133);
  pub const MIDNIGHTBLUE: Color = Self::from_rgb(25, 25, 112);
  pub const MINTCREAM: Color = Self::from_rgb(245, 255, 250);
  pub const MISTYROSE: Color = Self::from_rgb(255, 228, 225);

  pub const MOCCASIN: Color = Self::from_rgb(255, 228, 181);

  pub const NAVAJOWHITE: Color = Self::from_rgb(255, 222, 173);
  pub const NAVY: Color = Self::from_rgb(0, 0, 128);
  pub const OLDLACE: Color = Self::from_rgb(253, 245, 230);
  pub const OLIVE: Color = Self::from_rgb(128, 128, 0);

  pub const OLIVEDRAB: Color = Self::from_rgb(107, 142, 35);

  pub const ORANGE: Color = Self::from_rgb(255, 165, 0);
  pub const ORANGERED: Color = Self::from_rgb(255, 69, 0);
  pub const ORCHID: Color = Self::from_rgb(218, 112, 214);
  pub const PALEGOLDENROD: Color = Self::from_rgb(238, 232, 170);

  pub const PALEGREEN: Color = Self::from_rgb(152, 251, 152);

  pub const PALETURQUOISE: Color = Self::from_rgb(175, 238, 238);
  pub const PALEVIOLETRED: Color = Self::from_rgb(219, 112, 147);
  pub const PAPAYAWHIP: Color = Self::from_rgb(255, 239, 213);
  pub const PEACHPUFF: Color = Self::from_rgb(255, 218, 185);

  pub const PERU: Color = Self::from_rgb(205, 133, 63);

  pub const PINK: Color = Self::from_rgb(255, 192, 203);
  pub const PLUM: Color = Self::from_rgb(221, 160, 221);
  pub const POWDERBLUE: Color = Self::from_rgb(176, 224, 230);
  pub const PURPLE: Color = Self::from_rgb(128, 0, 128);

  pub const RED: Color = Self::from_rgb(255, 0, 0);

  pub const ROSYBROWN: Color = Self::from_rgb(188, 143, 143);
  pub const ROYALBLUE: Color = Self::from_rgb(65, 105, 225);
  pub const SADDLEBROWN: Color = Self::from_rgb(139, 69, 19);
  pub const SALMON: Color = Self::from_rgb(250, 128, 114);

  pub const SANDYBROWN: Color = Self::from_rgb(244, 164, 96);

  pub const SEAGREEN: Color = Self::from_rgb(46, 139, 87);
  pub const SEASHELL: Color = Self::from_rgb(255, 245, 238);
  pub const SIENNA: Color = Self::from_rgb(160, 82, 45);
  pub const SILVER: Color = Self::from_rgb(192, 192, 192);

  pub const SKYBLUE: Color = Self::from_rgb(135, 206, 235);

  pub const SLATEBLUE: Color = Self::from_rgb(106, 90, 205);
  pub const SLATEGRAY: Color = Self::from_rgb(112, 128, 144);
  pub const SLATEGREY: Color = Self::from_rgb(112, 128, 144);
  pub const SNOW: Color = Self::from_rgb(255, 250, 250);

  pub const SPRINGGREEN: Color = Self::from_rgb(0, 255, 127);

  pub const STEELBLUE: Color = Self::from_rgb(70, 130, 180);
  pub const TAN: Color = Self::from_rgb(210, 180, 140);
  pub const TEAL: Color = Self::from_rgb(0, 128, 128);
  pub const THISTLE: Color = Self::from_rgb(216, 191, 216);

  pub const TOMATO: Color = Self::from_rgb(255, 99, 71);

  pub const TURQUOISE: Color = Self::from_rgb(64, 224, 208);
  pub const VIOLET: Color = Self::from_rgb(238, 130, 238);
  pub const WHEAT: Color = Self::from_rgb(245, 222, 179);
  pub const WHITE: Color = Self::from_rgb(255, 255, 255);

  pub const WHITESMOKE: Color = Self::from_rgb(245, 245, 245);

  pub const YELLOW: Color = Self::from_rgb(255, 255, 0);
  pub const YELLOWGREEN: Color = Self::from_rgb(154, 205, 50);
  pub const TRANSPARENT: Color = Self::new(0, 0, 0, 0);
}
