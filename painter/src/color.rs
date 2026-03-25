use ribir_types::Point;
pub use ribir_types::{Color, LightnessTone};
use serde::{Deserialize, Serialize};

use crate::SpreadMethod;

/// The color filter matrix.
///
/// The effect of ColorFilterMatrix {matrix, base_color}, when apply to Color
/// of (R, G, B, A) will be: matrix * [R, G, B, A] + base_color,
/// and when base_color is None, the effect will be matrix * [R, G, B, A].
/// with matrix is:  | r1 r2 r3 r4 r5 | and base_color is: | r5 r5 r5 r5 |
///                  | g1 g2 g3 g4 g5 |
///                  | b1 b2 b3 b4 b5 |
///                  | a1 a2 a3 a4 a5 |
/// you can get color of (R', G', B', A') by:
///     R' = r1*R + r2*G + r3*B + r4*A + r5
///     G' = g1*R + g2*G + g3*B + g4*A + g5
///     B' = b1*R + b2*G + b3*B + b4*A + b5
///     A' = a1*R + a2*G + a3*B + a4*A + a5
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ColorFilterMatrix {
  /// The matrix for color filter. layout as 4 * 4:
  ///     | r1 r2 r3 r4 |
  ///     | g1 g2 g3 g4 |
  ///     | b1 b2 b3 b4 |
  ///     | a1 a2 a3 a4 |
  /// it is a row-major matrix. it will be used when apply to color(R, G, B, A)
  /// as:
  /// | R' |     | r1 r2 r3 r4 r5 |   | R |
  /// | G' |     | g1 g2 g3 g4 g5 |   | G |
  /// | B' |  =  | b1 b2 b3 b4 b5 | * | B |
  /// | A' |     | a1 a2 a3 a4 a5 |   | A |
  pub matrix: [f32; 16],

  /// The base color for color filter.
  /// it will be added to the result color(R', G', B', A') after the matrix
  /// applied
  pub base_color: Option<Color>,
}

#[inline]
fn dot(
  row: usize, col: usize, m1: &[f32], m2: &[f32], m1_row_cnt: usize, m2_row_cnt: usize,
) -> f32 {
  let (mut r, mut c, mut s) = (row * m1_row_cnt, col, 0.);
  for _ in 0..m1_row_cnt {
    s += m1[r] * m2[c];
    r += 1;
    c += m2_row_cnt;
  }
  s
}

impl ColorFilterMatrix {
  #[inline]
  pub fn new(matrix: [f32; 16]) -> Self { Self { matrix, base_color: None } }

  pub fn only_alpha(alpha: f32) -> Self {
    Self::new([
      1., 0., 0., 0., // red
      0., 1., 0., 0., // green
      0., 0., 1., 0., // blue
      0., 0., 0., alpha, // alpha
    ])
  }

  pub fn with_base(self, color: Color) -> Self { Self { base_color: Some(color), ..self } }

  pub fn chains(&mut self, next: &ColorFilterMatrix) -> ColorFilterMatrix {
    let mut matrix = [0.; 16];
    for (i, item) in matrix.iter_mut().enumerate() {
      *item = dot(i / 4, i % 4, &self.matrix, &next.matrix, 4, 4);
    }
    let mut base_color = next.base_color;
    if let Some(c) = self.base_color.as_ref() {
      let f = c.into_f32_components();
      let base = base_color
        .map(|c| c.into_f32_components())
        .unwrap_or([0.; 4]);

      base_color = Some(Color::from_f32_rgba(
        (base[0] + dot(0, 0, &next.matrix, &f, 4, 1)).clamp(0., 1.),
        (base[1] + dot(1, 0, &next.matrix, &f, 4, 1)).clamp(0., 1.),
        (base[2] + dot(2, 0, &next.matrix, &f, 4, 1)).clamp(0., 1.),
        (base[3] + dot(3, 0, &next.matrix, &f, 4, 1)).clamp(0., 1.),
      ));
    }
    Self { matrix, base_color }
  }

  pub fn apply_to(&self, color: &Color) -> Color {
    let c = color.into_f32_components();
    let base = self
      .base_color
      .as_ref()
      .map(|c| c.into_f32_components())
      .unwrap_or([0.; 4]);

    Color::from_f32_rgba(
      (base[0] + dot(0, 0, &self.matrix, &c, 4, 1)).clamp(0., 1.),
      (base[1] + dot(1, 0, &self.matrix, &c, 4, 1)).clamp(0., 1.),
      (base[2] + dot(2, 0, &self.matrix, &c, 4, 1)).clamp(0., 1.),
      (base[3] + dot(3, 0, &self.matrix, &c, 4, 1)).clamp(0., 1.),
    )
  }

  pub fn apply_alpha(&mut self, alpha: f32) {
    self.matrix[12] *= alpha;
    self.matrix[13] *= alpha;
    self.matrix[14] *= alpha;
    self.matrix[15] *= alpha;
    if let Some(color) = self.base_color.as_mut() {
      *color = color.apply_alpha(alpha);
    }
  }

  pub fn is_transparent(&self) -> bool {
    self
      .base_color
      .as_ref()
      .map(|c| c.alpha == 0)
      .unwrap_or(true)
      && self.matrix[12] == 0.
      && self.matrix[13] == 0.
      && self.matrix[14] == 0.
      && self.matrix[15] == 0.
  }
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
