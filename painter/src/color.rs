// todo: use u8 x4 replace f32
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Color {
  /// The amount of red light, where 0.0 is no red light and 1.0f
  /// is the highest displayable amount.
  pub red: f32,
  /// The amount of green light, where 0.0 is no green light and 1.0f  is the
  /// highest displayable amount.
  pub green: f32,
  /// The amount of blue light, where 0.0 is no blue light and 1.0f is the
  /// highest displayable amount.
  pub blue: f32,
  /// The amount pf transparency, where 0.0 is fully transparent and 1.0 is
  /// fully opaque.
  pub alpha: f32,
}

impl Color {
  #[inline]
  pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
    Color { red, green, blue, alpha }
  }

  pub fn into_raw(self) -> [u8; 4] {
    [
      Self::f32_to_u8(self.red),
      Self::f32_to_u8(self.green),
      Self::f32_to_u8(self.blue),
      Self::f32_to_u8(self.alpha),
    ]
  }

  pub fn as_u32(&self) -> u32 {
    (Self::f32_to_u8(self.red) as u32) << 24
      | (Self::f32_to_u8(self.green) as u32) << 16
      | (Self::f32_to_u8(self.blue) as u32) << 8
      | Self::f32_to_u8(self.alpha) as u32
  }

  #[inline]
  pub fn into_arrays(self) -> [f32; 4] { [self.red, self.green, self.blue, self.alpha] }

  pub fn from_u32(word: u32) -> Self {
    let [r, g, b, a]: [u8; 4] = word.to_be().to_ne_bytes();
    Self {
      red: Color::u8_to_f32(r),
      green: Color::u8_to_f32(g),
      blue: Color::u8_to_f32(b),
      alpha: Color::u8_to_f32(a),
    }
  }

  pub fn lighten(&mut self, amount: f32) -> &mut Self {
    self.red += amount;
    self.green += amount;
    self.blue += amount;
    self
  }

  pub fn with_alpha(mut self, alpha: f32) -> Self {
    self.alpha = alpha;
    self
  }

  // from css3 keywords: https://www.w3.org/wiki/CSS/Properties/color/keywords
  pub const ALICEBLUE: Color = Self::const_rgb_from(240, 248, 255);
  pub const ANTIQUEWHITE: Color = Self::const_rgb_from(250, 235, 215);
  pub const AQUA: Color = Self::const_rgb_from(0, 255, 255);
  pub const AQUAMARINE: Color = Self::const_rgb_from(127, 255, 212);

  pub const AZURE: Color = Self::const_rgb_from(240, 255, 255);

  pub const BEIGE: Color = Self::const_rgb_from(245, 245, 220);
  pub const BISQUE: Color = Self::const_rgb_from(255, 228, 196);
  pub const BLACK: Color = Self::const_rgb_from(0, 0, 0);
  pub const BLANCHEDALMOND: Color = Self::const_rgb_from(255, 235, 205);

  pub const BLUE: Color = Self::const_rgb_from(0, 0, 255);

  pub const BLUEVIOLET: Color = Self::const_rgb_from(138, 43, 226);
  pub const BROWN: Color = Self::const_rgb_from(165, 42, 42);
  pub const BURLYWOOD: Color = Self::const_rgb_from(222, 184, 135);
  pub const CADETBLUE: Color = Self::const_rgb_from(95, 158, 160);

  pub const CHARTREUSE: Color = Self::const_rgb_from(127, 255, 0);

  pub const CHOCOLATE: Color = Self::const_rgb_from(210, 105, 30);
  pub const CORAL: Color = Self::const_rgb_from(255, 127, 80);
  pub const CORNFLOWERBLUE: Color = Self::const_rgb_from(100, 149, 237);
  pub const CORNSILK: Color = Self::const_rgb_from(255, 248, 220);

  pub const CRIMSON: Color = Self::const_rgb_from(220, 20, 60);

  pub const CYAN: Color = Self::const_rgb_from(0, 255, 255);
  pub const DARKBLUE: Color = Self::const_rgb_from(0, 0, 139);
  pub const DARKCYAN: Color = Self::const_rgb_from(0, 139, 139);
  pub const DARKGOLDENROD: Color = Self::const_rgb_from(184, 134, 11);

  pub const DARKGRAY: Color = Self::const_rgb_from(169, 169, 169);

  pub const DARKGREEN: Color = Self::const_rgb_from(0, 100, 0);
  pub const DARKGREY: Color = Self::const_rgb_from(169, 169, 169);
  pub const DARKKHAKI: Color = Self::const_rgb_from(189, 183, 107);
  pub const DARKMAGENTA: Color = Self::const_rgb_from(139, 0, 139);

  pub const DARKOLIVEGREEN: Color = Self::const_rgb_from(85, 107, 47);

  pub const DARKORANGE: Color = Self::const_rgb_from(255, 140, 0);
  pub const DARKORCHID: Color = Self::const_rgb_from(153, 50, 204);
  pub const DARKRED: Color = Self::const_rgb_from(139, 0, 0);
  pub const DARKSALMON: Color = Self::const_rgb_from(233, 150, 122);

  pub const DARKSEAGREEN: Color = Self::const_rgb_from(143, 188, 143);

  pub const DARKSLATEBLUE: Color = Self::const_rgb_from(72, 61, 139);
  pub const DARKSLATEGRAY: Color = Self::const_rgb_from(47, 79, 79);
  pub const DARKSLATEGREY: Color = Self::const_rgb_from(47, 79, 79);
  pub const DARKTURQUOISE: Color = Self::const_rgb_from(0, 206, 209);

  pub const DARKVIOLET: Color = Self::const_rgb_from(148, 0, 211);

  pub const DEEPPINK: Color = Self::const_rgb_from(255, 20, 147);
  pub const DEEPSKYBLUE: Color = Self::const_rgb_from(0, 191, 255);
  pub const DIMGRAY: Color = Self::const_rgb_from(105, 105, 105);
  pub const DIMGREY: Color = Self::const_rgb_from(105, 105, 105);

  pub const DODGERBLUE: Color = Self::const_rgb_from(30, 144, 255);

  pub const FIREBRICK: Color = Self::const_rgb_from(178, 34, 34);
  pub const FLORALWHITE: Color = Self::const_rgb_from(255, 250, 240);
  pub const FORESTGREEN: Color = Self::const_rgb_from(34, 139, 34);
  pub const FUCHSIA: Color = Self::const_rgb_from(255, 0, 255);

  pub const GAINSBORO: Color = Self::const_rgb_from(220, 220, 220);

  pub const GHOSTWHITE: Color = Self::const_rgb_from(248, 248, 255);
  pub const GOLD: Color = Self::const_rgb_from(255, 215, 0);
  pub const GOLDENROD: Color = Self::const_rgb_from(218, 165, 32);
  pub const GRAY: Color = Self::const_rgb_from(128, 128, 128);

  pub const GREEN: Color = Self::const_rgb_from(0, 128, 0);

  pub const GREENYELLOW: Color = Self::const_rgb_from(173, 255, 47);
  pub const GREY: Color = Self::const_rgb_from(128, 128, 128);
  pub const HONEYDEW: Color = Self::const_rgb_from(240, 255, 240);
  pub const HOTPINK: Color = Self::const_rgb_from(255, 105, 180);

  pub const INDIANRED: Color = Self::const_rgb_from(205, 92, 92);

  pub const INDIGO: Color = Self::const_rgb_from(75, 0, 130);
  pub const IVORY: Color = Self::const_rgb_from(255, 255, 240);
  pub const KHAKI: Color = Self::const_rgb_from(240, 230, 140);
  pub const LAVENDER: Color = Self::const_rgb_from(230, 230, 250);

  pub const LAVENDERBLUSH: Color = Self::const_rgb_from(255, 240, 245);

  pub const LAWNGREEN: Color = Self::const_rgb_from(124, 252, 0);
  pub const LEMONCHIFFON: Color = Self::const_rgb_from(255, 250, 205);
  pub const LIGHTBLUE: Color = Self::const_rgb_from(173, 216, 230);
  pub const LIGHTCORAL: Color = Self::const_rgb_from(240, 128, 128);

  pub const LIGHTCYAN: Color = Self::const_rgb_from(224, 255, 255);

  pub const LIGHTGOLDENRODYELLOW: Color = Self::const_rgb_from(250, 250, 210);
  pub const LIGHTGRAY: Color = Self::const_rgb_from(211, 211, 211);
  pub const LIGHTGREEN: Color = Self::const_rgb_from(144, 238, 144);
  pub const LIGHTGREY: Color = Self::const_rgb_from(211, 211, 211);

  pub const LIGHTPINK: Color = Self::const_rgb_from(255, 182, 193);

  pub const LIGHTSALMON: Color = Self::const_rgb_from(255, 160, 122);
  pub const LIGHTSEAGREEN: Color = Self::const_rgb_from(32, 178, 170);
  pub const LIGHTSKYBLUE: Color = Self::const_rgb_from(135, 206, 250);
  pub const LIGHTSLATEGRAY: Color = Self::const_rgb_from(119, 136, 153);

  pub const LIGHTSLATEGREY: Color = Self::const_rgb_from(119, 136, 153);

  pub const LIGHTSTEELBLUE: Color = Self::const_rgb_from(176, 196, 222);
  pub const LIGHTYELLOW: Color = Self::const_rgb_from(255, 255, 224);
  pub const LIME: Color = Self::const_rgb_from(0, 255, 0);
  pub const LIMEGREEN: Color = Self::const_rgb_from(50, 205, 50);

  pub const LINEN: Color = Self::const_rgb_from(250, 240, 230);

  pub const MAGENTA: Color = Self::const_rgb_from(255, 0, 255);
  pub const MAROON: Color = Self::const_rgb_from(128, 0, 0);
  pub const MEDIUMAQUAMARINE: Color = Self::const_rgb_from(102, 205, 170);
  pub const MEDIUMBLUE: Color = Self::const_rgb_from(0, 0, 205);

  pub const MEDIUMORCHID: Color = Self::const_rgb_from(186, 85, 211);

  pub const MEDIUMPURPLE: Color = Self::const_rgb_from(147, 112, 219);
  pub const MEDIUMSEAGREEN: Color = Self::const_rgb_from(60, 179, 113);
  pub const MEDIUMSLATEBLUE: Color = Self::const_rgb_from(123, 104, 238);
  pub const MEDIUMSPRINGGREEN: Color = Self::const_rgb_from(0, 250, 154);

  pub const MEDIUMTURQUOISE: Color = Self::const_rgb_from(72, 209, 204);

  pub const MEDIUMVIOLETRED: Color = Self::const_rgb_from(199, 21, 133);
  pub const MIDNIGHTBLUE: Color = Self::const_rgb_from(25, 25, 112);
  pub const MINTCREAM: Color = Self::const_rgb_from(245, 255, 250);
  pub const MISTYROSE: Color = Self::const_rgb_from(255, 228, 225);

  pub const MOCCASIN: Color = Self::const_rgb_from(255, 228, 181);

  pub const NAVAJOWHITE: Color = Self::const_rgb_from(255, 222, 173);
  pub const NAVY: Color = Self::const_rgb_from(0, 0, 128);
  pub const OLDLACE: Color = Self::const_rgb_from(253, 245, 230);
  pub const OLIVE: Color = Self::const_rgb_from(128, 128, 0);

  pub const OLIVEDRAB: Color = Self::const_rgb_from(107, 142, 35);

  pub const ORANGE: Color = Self::const_rgb_from(255, 165, 0);
  pub const ORANGERED: Color = Self::const_rgb_from(255, 69, 0);
  pub const ORCHID: Color = Self::const_rgb_from(218, 112, 214);
  pub const PALEGOLDENROD: Color = Self::const_rgb_from(238, 232, 170);

  pub const PALEGREEN: Color = Self::const_rgb_from(152, 251, 152);

  pub const PALETURQUOISE: Color = Self::const_rgb_from(175, 238, 238);
  pub const PALEVIOLETRED: Color = Self::const_rgb_from(219, 112, 147);
  pub const PAPAYAWHIP: Color = Self::const_rgb_from(255, 239, 213);
  pub const PEACHPUFF: Color = Self::const_rgb_from(255, 218, 185);

  pub const PERU: Color = Self::const_rgb_from(205, 133, 63);

  pub const PINK: Color = Self::const_rgb_from(255, 192, 203);
  pub const PLUM: Color = Self::const_rgb_from(221, 160, 221);
  pub const POWDERBLUE: Color = Self::const_rgb_from(176, 224, 230);
  pub const PURPLE: Color = Self::const_rgb_from(128, 0, 128);

  pub const RED: Color = Self::const_rgb_from(255, 0, 0);

  pub const ROSYBROWN: Color = Self::const_rgb_from(188, 143, 143);
  pub const ROYALBLUE: Color = Self::const_rgb_from(65, 105, 225);
  pub const SADDLEBROWN: Color = Self::const_rgb_from(139, 69, 19);
  pub const SALMON: Color = Self::const_rgb_from(250, 128, 114);

  pub const SANDYBROWN: Color = Self::const_rgb_from(244, 164, 96);

  pub const SEAGREEN: Color = Self::const_rgb_from(46, 139, 87);
  pub const SEASHELL: Color = Self::const_rgb_from(255, 245, 238);
  pub const SIENNA: Color = Self::const_rgb_from(160, 82, 45);
  pub const SILVER: Color = Self::const_rgb_from(192, 192, 192);

  pub const SKYBLUE: Color = Self::const_rgb_from(135, 206, 235);

  pub const SLATEBLUE: Color = Self::const_rgb_from(106, 90, 205);
  pub const SLATEGRAY: Color = Self::const_rgb_from(112, 128, 144);
  pub const SLATEGREY: Color = Self::const_rgb_from(112, 128, 144);
  pub const SNOW: Color = Self::const_rgb_from(255, 250, 250);

  pub const SPRINGGREEN: Color = Self::const_rgb_from(0, 255, 127);

  pub const STEELBLUE: Color = Self::const_rgb_from(70, 130, 180);
  pub const TAN: Color = Self::const_rgb_from(210, 180, 140);
  pub const TEAL: Color = Self::const_rgb_from(0, 128, 128);
  pub const THISTLE: Color = Self::const_rgb_from(216, 191, 216);

  pub const TOMATO: Color = Self::const_rgb_from(255, 99, 71);

  pub const TURQUOISE: Color = Self::const_rgb_from(64, 224, 208);
  pub const VIOLET: Color = Self::const_rgb_from(238, 130, 238);
  pub const WHEAT: Color = Self::const_rgb_from(245, 222, 179);
  pub const WHITE: Color = Self::const_rgb_from(255, 255, 255);

  pub const WHITESMOKE: Color = Self::const_rgb_from(245, 245, 245);

  pub const YELLOW: Color = Self::const_rgb_from(255, 255, 0);
  pub const YELLOWGREEN: Color = Self::const_rgb_from(154, 205, 50);
  pub const TRANSPARENT: Color = Self {
    alpha: 0.,
    red: 0.,
    green: 0.,
    blue: 0.,
  };

  // Algorithm from https://github.com/Ogeon/palette/pull/184/files.
  fn u8_to_f32(v: u8) -> f32 {
    let comp_u = v as u32 + C23;
    let comp_f = f32::from_bits(comp_u) - f32::from_bits(C23);
    let max_u = core::u8::MAX as u32 + C23;
    let max_f = (f32::from_bits(max_u) - f32::from_bits(C23)).recip();
    comp_f * max_f
  }

  // Algorithm from https://github.com/Ogeon/palette/pull/184/files.
  fn f32_to_u8(v: f32) -> u8 {
    let max = u8::MAX as f32;
    let scaled = (v * max).min(max);
    let f = scaled + f32::from_bits(C23);
    (f.to_bits().saturating_sub(C23)) as u8
  }

  const fn const_rgb_from(red: u8, green: u8, blue: u8) -> Self {
    Color {
      red: red as f32 / u8::MAX as f32,
      green: green as f32 / u8::MAX as f32,
      blue: blue as f32 / u8::MAX as f32,
      alpha: 1.0,
    }
  }
}

const C23: u32 = 0x4b00_0000;

#[cfg(test)]
mod tests {
  use super::*;

  extern crate test;
  use test::Bencher;

  #[test]
  fn component_convert() {
    fn convert(c: f32) -> f32 { Color::u8_to_f32(Color::f32_to_u8(c)) }

    assert!(convert(0.).abs() <= f32::EPSILON);
    assert!((convert(1.0) - 1.0).abs() <= f32::EPSILON);
    assert!(convert(-1.).abs() <= f32::EPSILON);
    assert!((convert(f32::NAN) - 1.) <= f32::EPSILON);
  }

  #[test]
  fn as_u32() {
    assert_eq!(Color::BLACK.as_u32(), 0x0000_00FF);
    assert_eq!(Color::RED.as_u32(), 0xFF00_00FF);
  }

  #[test]
  fn lighten() {
    let mut black = Color::BLACK;
    assert_eq!(black.lighten(0.1).as_u32(), 0x1A1A_1AFF);
    assert_eq!(black.lighten(0.1).as_u32(), 0x3333_33FF);
    assert_eq!(black.lighten(0.1).as_u32(), 0x4C4C_4CFF);
    assert_eq!(black.lighten(1.).as_u32(), 0xFFFF_FFFF);
  }

  #[bench]
  fn f32_to_u8(b: &mut Bencher) {
    b.iter(|| {
      let sum: u32 = (0..100).map(|i| Color::f32_to_u8(i as f32) as u32).sum();
      sum
    })
  }

  #[bench]
  fn u8_to_f32(b: &mut Bencher) {
    b.iter(|| {
      let sum: f32 = (0..100).map(|i| Color::u8_to_f32(i as u8)).sum();
      sum
    })
  }
}
