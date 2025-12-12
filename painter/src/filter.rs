//! Filter types and utilities for applying visual effects.

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::color::{Color, ColorFilterMatrix};

/// Represents a 2D convolution matrix used for image filtering operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlattenMatrix {
  pub width: usize,
  pub height: usize,
  pub matrix: Vec<f32>,
}

/// The operation type of the filter processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterOp {
  Color(ColorFilterMatrix),
  Convolution(FlattenMatrix),
}

/// A filter layer that contains a list of operations, a composite mode, and an
/// offset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterLayer {
  pub ops: SmallVec<[FilterOp; 1]>,
  pub composite: FilterComposite,
  /// Sample offset [dx, dy] for filter operations.
  /// Positive values shift the sampled content, creating effects like drop
  /// shadows.
  #[serde(default)]
  pub offset: [f32; 2],
}

/// The composite type for the filter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum FilterComposite {
  #[default]
  Replace,
  /// The filter result will only be applied to the area where the alpha of the
  /// source is 0.
  ExcludeSource,
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
/// let combined = Filter::grayscale(0.5).then(Filter::blur(3.0));
/// ```
#[derive(Default, Clone, Debug)]
pub struct Filter {
  pub layers: SmallVec<[FilterLayer; 1]>,
}

impl Filter {
  /// Create an empty filter
  pub fn new() -> Self { Self { layers: SmallVec::new() } }

  /// Combines two filters by extending the current filter with another.
  ///
  /// This method is useful for chaining multiple filters together.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ribir_painter::{Color, Filter};
  ///
  /// // Chain multiple filters together
  /// let combined = Filter::grayscale(0.5).then(Filter::blur(3.0));
  /// ```
  pub fn then(mut self, filter: Self) -> Self {
    if filter.is_empty() {
      return self;
    }

    if self.is_empty() {
      return filter;
    }

    // Try to merge the first layer of the new filter into the last layer of the
    // current filter
    let can_merge = {
      let last = self.layers.last().unwrap();
      let first = filter.layers.first().unwrap();
      last.composite == FilterComposite::Replace
        && last.offset == [0., 0.]
        && first.composite == FilterComposite::Replace
        && first.offset == [0., 0.]
    };

    if can_merge {
      let mut iter = filter.layers.into_iter();
      let first = iter.next().unwrap();
      self
        .layers
        .last_mut()
        .unwrap()
        .ops
        .extend(first.ops);
      self.layers.extend(iter);
    } else {
      self.layers.extend(filter.layers);
    }
    self
  }

  /// Sets the composite operation of the last filter stage.
  pub fn composite_op(mut self, composite: FilterComposite) -> Self {
    if let Some(layer) = self.layers.last_mut() {
      layer.composite = composite;
    }
    self
  }

  /// Sets the offset of the last filter stage.
  /// The offset specifies the sample displacement [dx, dy] for filter
  /// operations. Positive values shift the sampled content, creating effects
  /// like drop shadows.
  pub fn offset(mut self, dx: f32, dy: f32) -> Self {
    if let Some(layer) = self.layers.last_mut() {
      layer.offset = [dx, dy];
    }
    self
  }

  /// Creates a grayscale filter with the specified amount.
  /// Amount should be between 0.0 and 1.0, where 1.0 is fully grayscale.
  #[rustfmt::skip]
  #[rustfmt::skip]
  pub fn grayscale(amount: f32) -> Self {
    let t = amount.clamp(0.0, 1.0);
    let (r, g, b) = (0.2126, 0.7152, 0.0722);
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            1.0 - t + t * r,   t * g,             t * b,             0.0, // red
            t * r,             1.0 - t + t * g,   t * b,             0.0, // green
            t * r,             t * g,             1.0 - t + t * b,   0.0, // blue
            0.0,               0.0,               0.0,               1.0, // alpha
          ],
          base_color: None,
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a sepia filter with the specified amount.
  /// Amount should be between 0.0 and 1.0, where 1.0 is fully sepia.
  #[rustfmt::skip]
  pub fn sepia(amount: f32) -> Self {
    let t = amount.clamp(0.0, 1.0);
    // Sepia weights from W3C Filter Effects Module Level 1
    // https://www.w3.org/TR/filter-effects-1/#sepiaEquivalent
    let (r0, r1, r2) = (0.393, 0.769, 0.189);
    let (g0, g1, g2) = (0.349, 0.686, 0.168);
    let (b0, b1, b2) = (0.272, 0.534, 0.131);

    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            1.0 - t + t * r0,  t * r1,            t * r2,            0.0, // red
            t * g0,            1.0 - t + t * g1,  t * g2,            0.0, // green
            t * b0,            t * b1,            1.0 - t + t * b2,  0.0, // blue
            0.0,               0.0,               0.0,               1.0, // alpha
          ],
          base_color: None,
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a saturation filter.
  /// Level < 0.5 desaturates, level > 0.5 saturates, level = 1.0 maintains
  /// original.
  #[rustfmt::skip]
  pub fn saturate(level: f32) -> Self {
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            0.213 + 0.787 * level, 0.715 - 0.715 * level, 0.072 - 0.072 * level, 0.,  // red
            0.213 - 0.213 * level, 0.715 + 0.285 * level, 0.072 - 0.072 * level, 0.,  // green
            0.213 - 0.213 * level, 0.715 - 0.715 * level, 0.072 + 0.928 * level, 0.,  // blue
            0., 0., 0., 1.,  // alpha
          ],
          base_color: None,
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates an opacity filter.
  /// Amount should be between 0.0 (transparent) and 1.0 (opaque).
  #[rustfmt::skip]
  pub fn opacity(amount: f32) -> Self {
    let v = amount.clamp(0.0, 1.0);
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, v
          ],
          base_color: None,
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a contrast filter.
  /// Amount should be between 0.0 (no contrast) and 1.0 (maximum contrast).
  pub fn contrast(amount: f32) -> Self {
    let c = amount.clamp(0.0, 1.0);
    let color_offset = 0.5 * (1.0 - c);
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            c, 0.0, 0.0, 0.0, // R
            0.0, c, 0.0, 0.0, // G
            0.0, 0.0, c, 0.0, // B
            0.0, 0.0, 0.0, 1.0, // A
          ],
          base_color: Some(Color::from_f32_rgba(color_offset, color_offset, color_offset, 0.0)),
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a brightness filter.
  /// Amount = 1.0 is no change, < 1.0 darkens, > 1.0 brightens.
  #[rustfmt::skip]
  pub fn brightness(amount: f32) -> Self {
    let t = (amount - 1.0).max(-1.0);
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            1.,   0.0,  0.0,  0.0,
            0.0,  1.,   0.0,  0.0,
            0.0,  0.0,  1.,   0.0,
            0.0,  0.0,  0.0,  1.0,
          ],
          base_color: Some(Color::from_f32_rgba(t, t, t, 0.0)),
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
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
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(matrix)],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
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
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(matrix)],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a luminance to alpha filter.
  pub fn luminance_to_alpha() -> Self {
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(ColorFilterMatrix {
          matrix: [
            0., 0., 0., 0., // red
            0., 0., 0., 0., // green
            0., 0., 0., 0., // blue
            0.2125, 0.7154, 0.0721, 0., // alpha
          ],
          base_color: None,
        })],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
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
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![
          FilterOp::Convolution(FlattenMatrix {
            width: kernel_len,
            height: 1,
            matrix: kernel.clone(),
          }),
          FilterOp::Convolution(FlattenMatrix { width: 1, height: kernel_len, matrix: kernel })
        ],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      },],
    }
  }

  /// Creates a color filter with the specified color matrix.
  pub fn color(matrix: ColorFilterMatrix) -> Self {
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Color(matrix)],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a convolution filter with the specified matrix.
  pub fn convolution(matrix: FlattenMatrix) -> Self {
    Self {
      layers: smallvec![FilterLayer {
        ops: smallvec![FilterOp::Convolution(matrix)],
        composite: FilterComposite::default(),
        offset: [0., 0.],
      }],
    }
  }

  /// Creates a drop-shadow filter.
  ///
  /// The drop-shadow effect renders a blurred, offset shadow behind the source
  /// content. The shadow color replaces the source colors while preserving the
  /// alpha channel.
  ///
  /// # Arguments
  /// * `offset` - The shadow offset (dx, dy) in pixels. Positive values shift
  ///   the shadow right and down.
  /// * `blur_radius` - The blur radius for the shadow. Use 0.0 for a sharp
  ///   shadow.
  /// * `shadow_color` - The color of the shadow.
  ///
  /// # Example
  /// ```ignore
  /// // Create a drop shadow offset 5px right and 5px down, with 3px blur
  /// let shadow = Filter::drop_shadow((5.0, 5.0), 3.0, Color::from_f32_rgba(0.0, 0.0, 0.0, 0.5));
  /// ```
  pub fn drop_shadow(offset: (f32, f32), blur_radius: f32, shadow_color: Color) -> Self {
    // 1. Shadow color matrix
    let shadow_matrix = shadow_color_matrix(shadow_color);

    // 2. Blur operations
    let mut ops = smallvec![FilterOp::Color(shadow_matrix)];

    if blur_radius > 0.001 {
      let radius_usize = blur_radius.ceil() as usize;
      let radius_usize = radius_usize.min(30);
      let kernel = gaussian_kernel(radius_usize, blur_radius / 2.);
      let kernel_len = kernel.len();

      ops.push(FilterOp::Convolution(FlattenMatrix {
        width: kernel_len,
        height: 1,
        matrix: kernel.clone(),
      }));
      ops.push(FilterOp::Convolution(FlattenMatrix {
        width: 1,
        height: kernel_len,
        matrix: kernel,
      }));
    }

    Self {
      layers: smallvec![FilterLayer {
        ops,
        composite: FilterComposite::ExcludeSource,
        offset: [offset.0, offset.1],
      }],
    }
  }

  /// Returns true if the filter contains no filter types.
  pub fn is_empty(&self) -> bool { self.layers.is_empty() }

  /// Returns the number of filter types in the filter.
  pub fn len(&self) -> usize { self.layers.len() }

  /// Converts the filter into a Vec of filter primitives.
  pub(crate) fn into_layers(self) -> Vec<FilterLayer> { self.layers.into_vec() }

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
    let mut layers = SmallVec::new();
    let mut optimization_broken = false;

    for mut layer in self.layers {
      if !optimization_broken
        && layer.offset == [0., 0.]
        && layer.composite == FilterComposite::Replace
      {
        let mut new_ops = SmallVec::new();
        for op in layer.ops {
          if !optimization_broken {
            match op {
              FilterOp::Color(m) => {
                // Extract Color Op
                match &mut color_matrix {
                  Some(existing) => *existing = existing.chains(&m),
                  None => color_matrix = Some(m),
                }
                continue; // Op extracted, don't add to new_ops
              }
              _ => {
                // Encountered non-color op, stop optimization
                optimization_broken = true;
                new_ops.push(op);
              }
            }
          } else {
            // Optimization already broken, keep remaining ops
            new_ops.push(op);
          }
        }

        if !new_ops.is_empty() {
          layer.ops = new_ops;
          layers.push(layer);
        }
        // If new_ops is empty, the whole layer was consumed (all Color ops)
      } else {
        optimization_broken = true;
        layers.push(layer);
      }
    }

    (color_matrix, Filter { layers })
  }
}

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

/// Creates a color filter matrix that converts any color to the specified
/// shadow color while preserving the source alpha.
///
/// The matrix ignores the input RGB values and outputs the shadow color's RGB,
/// while the output alpha is the product of the input alpha and the shadow
/// color's alpha.
///
/// # Arguments
/// * `color` - The shadow color to use.
///
/// # Returns
/// A `ColorFilterMatrix` that transforms any color to the shadow color.
#[rustfmt::skip]
fn shadow_color_matrix(color: Color) -> ColorFilterMatrix {
  let [r, g, b, a] = color.into_f32_components();
  ColorFilterMatrix {
    matrix: [
      0.0, 0.0, 0.0, 0.0,  // R: ignore input, use base_color
      0.0, 0.0, 0.0, 0.0,  // G: ignore input, use base_color
      0.0, 0.0, 0.0, 0.0,  // B: ignore input, use base_color
      0.0, 0.0, 0.0, a,    // A: preserve input alpha × shadow alpha
    ],
    base_color: Some(Color::from_f32_rgba(r, g, b, 0.0)),
  }
}
