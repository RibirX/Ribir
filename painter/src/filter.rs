//! Filter types and utilities for applying visual effects.

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::color::{Color, ColorFilterMatrix};

/// Represents a 2D convolution matrix used for image filtering operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlattenMatrix {
  pub width: usize,
  pub height: usize,
  pub matrix: Vec<f32>,
}

/// The type of filter to apply to the content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterType {
  Color(ColorFilterMatrix),
  Convolution(FlattenMatrix),
}

/// A filter that can be applied to painted content.
///
/// Filter provides a fluent API for creating various filter effects like blur,
/// grayscale, brightness, contrast, etc.
///
/// # Example
/// ```ignore
/// // Create a blur filter with radius 5
/// let blur = Filter::blur(5.0);
///
/// // Chain multiple filters together
/// let combined = Filter::grayscale(0.5).with(Filter::blur(3.0));
/// ```
#[derive(Default, Clone, Debug)]
pub struct Filter(SmallVec<[FilterType; 2]>);

impl Filter {
  /// Create an empty filter
  pub fn new() -> Self { Self(SmallVec::new()) }

  /// Combines two filters by extending the current filter with another.
  pub fn with(mut self, filter: Self) -> Self {
    self.0.extend(filter.0);
    self
  }

  /// Creates a grayscale filter with the specified amount.
  /// Amount should be between 0.0 and 1.0, where 1.0 is fully grayscale.
  #[rustfmt::skip]
  pub fn grayscale(amount: f32) -> Self {
    let t = amount.clamp(0.0, 1.0);
    let (r, g, b) = (0.2126, 0.7152, 0.0722);
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        1.0 - t + t * r,   t * g,             t * b,             0.0, // red
        t * r,             1.0 - t + t * g,   t * b,             0.0, // green
        t * r,             t * g,             1.0 - t + t * b,   0.0, // blue
        0.0,               0.0,               0.0,               1.0, // alpha
      ],
      base_color: None,
    })])
  }

  /// Creates a saturation filter.
  /// Level < 0.5 desaturates, level > 0.5 saturates, level = 1.0 maintains
  /// original.
  #[rustfmt::skip]
  pub fn saturate(level: f32) -> Self {
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        0.213 + 0.787 * level, 0.715 - 0.715 * level, 0.072 - 0.072 * level, 0.,  // red
        0.213 - 0.213 * level, 0.715 + 0.285 * level, 0.072 - 0.072 * level, 0.,  // green
        0.213 - 0.213 * level, 0.715 - 0.715 * level, 0.072 + 0.928 * level, 0.,  // blue
        0., 0., 0., 1.,  // alpha
      ],
      base_color: None,
    })])
  }

  /// Creates an opacity filter.
  /// Amount should be between 0.0 (transparent) and 1.0 (opaque).
  #[rustfmt::skip]
  pub fn opacity(amount: f32) -> Self {
    let v = amount.clamp(0.0, 1.0);
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, v
      ],
      base_color: None,
    })])
  }

  /// Creates a contrast filter.
  /// Amount should be between 0.0 (no contrast) and 1.0 (maximum contrast).
  pub fn contrast(amount: f32) -> Self {
    let c = amount.clamp(0.0, 1.0);
    let offset = 0.5 * (1.0 - c);
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        c, 0.0, 0.0, 0.0, // R
        0.0, c, 0.0, 0.0, // G
        0.0, 0.0, c, 0.0, // B
        0.0, 0.0, 0.0, 1.0, // A
      ],
      base_color: Some(Color::from_f32_rgba(offset, offset, offset, 0.0)),
    })])
  }

  /// Creates a brightness filter.
  /// Amount = 1.0 is no change, < 1.0 darkens, > 1.0 brightens.
  #[rustfmt::skip]
  pub fn brightness(amount: f32) -> Self {
    let t = (amount - 1.0).max(-1.0);
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        1.,   0.0,  0.0,  0.0,
        0.0,  1.,   0.0,  0.0,
        0.0,  0.0,  1.,   0.0,
        0.0,  0.0,  0.0,  1.0,
      ],
      base_color: Some(Color::from_f32_rgba(t, t, t, 0.0)),
    })])
  }

  /// Creates a hue rotation filter.
  /// Angle is in radians.
  #[rustfmt::skip]
  pub fn hue_rotate(rad: f32) -> Self {
    let matrix = ColorFilterMatrix {
      matrix: [
        // red
        0.213 + rad.cos() * 0.787 - rad.sin() * 0.213,
        0.715 - rad.cos() * 0.715 - rad.sin() * 0.715,
        0.072 - rad.cos() * 0.072 + rad.sin() * 0.928,
        0.,

        // green
        0.213 - rad.cos() * 0.213 + rad.sin() * 0.143,
        0.715 + rad.cos() * 0.285 + rad.sin() * 0.14,
        0.072 - rad.cos() * 0.072 - rad.sin() * 0.283,
        0.,

        // blue
        0.213 - rad.cos() * 0.213 - rad.sin() * 0.787,
        0.715 - rad.cos() * 0.715 + rad.sin() * 0.715,
        0.072 + rad.cos() * 0.928 + rad.sin() * 0.072,
        0.,

        // alpha
        0., 0., 0.,1.,
      ],
      base_color: None,
    };
    Self(smallvec![FilterType::Color(matrix)])
  }

  /// Creates an invert filter.
  /// Amount should be between 0.0 (no inversion) and 1.0 (full inversion).
  pub fn invert(amount: f32) -> Self {
    let i = amount.clamp(0.0, 1.0);
    #[rustfmt::skip]
    let matrix = ColorFilterMatrix {
      matrix: [
        1. - 2. * i,    0.0,             0.0,             0.0,
        0.0,            1. - 2. * i,     0.0,             0.0,
        0.0,            0.0,             1. - 2. * i,     0.0,
        0.0,            0.0,             0.0,             1.0,
      ],
      base_color: Some(Color::from_f32_rgba(i, i, i, 0.)),
    };
    Self(smallvec![FilterType::Color(matrix)])
  }

  /// Creates a luminance to alpha filter.
  pub fn luminance_to_alpha() -> Self {
    Self(smallvec![FilterType::Color(ColorFilterMatrix {
      matrix: [
        0., 0., 0., 0., // red
        0., 0., 0., 0., // green
        0., 0., 0., 0., // blue
        0.2125, 0.7154, 0.0721, 0., // alpha
      ],
      base_color: None,
    })])
  }

  /// Creates a blur filter with the specified radius.
  /// Note that the radius should be less than or equal to 30.
  pub fn blur(radius: f32) -> Self {
    if radius <= 0.001 {
      // Using small epsilon to handle floating point precision
      return Self::default();
    }

    let radius_usize = radius.ceil() as usize;
    let radius_usize = radius_usize.min(30);
    let kernel = gaussian_kernel(radius_usize, radius / 2.);
    let kernel_len = kernel.len();
    Self(smallvec![
      FilterType::Convolution(FlattenMatrix {
        width: kernel_len,
        height: 1,
        matrix: kernel.clone(),
      }),
      FilterType::Convolution(FlattenMatrix { width: 1, height: kernel_len, matrix: kernel }),
    ])
  }

  /// Creates a color filter with the specified color matrix.
  pub fn color(matrix: ColorFilterMatrix) -> Self { Self(smallvec![FilterType::Color(matrix)]) }

  /// Creates a convolution filter with the specified matrix.
  pub fn convolution(matrix: FlattenMatrix) -> Self {
    Self(smallvec![FilterType::Convolution(matrix)])
  }

  /// Returns true if the filter contains no filter types.
  pub fn is_empty(&self) -> bool { self.0.is_empty() }

  /// Returns the number of filter types in the filter.
  pub fn len(&self) -> usize { self.0.len() }

  /// Converts the filter into a Vec of filter types.
  pub(crate) fn into_vec(self) -> Vec<FilterType> { self.0.into_vec() }

  /// Extracts the combined color filter matrix and remaining convolution
  /// filters.
  ///
  /// This method separates color filters from convolution filters. All color
  /// filters are chained together into a single `ColorFilterMatrix`, and the
  /// convolution filters are collected into a new `Filter`.
  ///
  /// Returns a tuple of:
  /// - `Option<ColorFilterMatrix>`: The combined color filter matrix, or `None`
  ///   if no color filters
  /// - `Filter`: The remaining convolution filters
  pub(crate) fn extract_color_and_convolution(self) -> (Option<ColorFilterMatrix>, Filter) {
    let mut color_matrix: Option<ColorFilterMatrix> = None;
    let mut convolutions = SmallVec::new();

    for filter in self.0 {
      match filter {
        FilterType::Color(matrix) => {
          color_matrix = Some(match color_matrix {
            Some(mut existing) => existing.chains(&matrix),
            None => matrix,
          });
        }
        FilterType::Convolution(conv) => {
          convolutions.push(FilterType::Convolution(conv));
        }
      }
    }

    (color_matrix, Filter(convolutions))
  }
}

/// Generates a Gaussian 1 dimension kernel with the specified radius and sigma.
/// The sigma must be greater than 0.
fn gaussian_kernel(radius: usize, sigma: f32) -> Vec<f32> {
  let size = 2 * radius + 1;

  let mut kernel = Vec::with_capacity(size);
  let mut sum = 0.0;

  for i in 0..=radius {
    let x = i as f32 - radius as f32;
    let weight = (-x.powi(2) / (2.0 * sigma.powi(2))).exp();
    sum += weight;
    kernel.push(weight);
  }

  for i in 1..=radius {
    let weight = kernel[radius - i];
    sum += weight;
    kernel.push(weight);
  }

  let reciprocal = 1.0 / sum;
  kernel.iter_mut().for_each(|w| *w *= reciprocal);
  kernel
}
