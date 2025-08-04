use ribir_painter::color::ColorFilterMatrix;

use crate::{prelude::*, wrap_render::WrapRender};

/// GRAYSCALE_FILTER
///
/// GRAYSCALE_FILTER will convert the color value of each pixel in an image to
/// grayscale.
pub const GRAYSCALE_FILTER: ColorFilterMatrix = ColorFilterMatrix {
  matrix: [
    0.2126, 0.7152, 0.0722, 0., // red
    0.2126, 0.7152, 0.0722, 0., // green
    0.2126, 0.7152, 0.0722, 0., // blue
    0., 0., 0., 1., // alpha
  ],
  base_color: None,
};

/// LUMINANCE_TO_ALPHA_FILTER
///
/// LUMINANCE_TO_ALPHA_FILTER will convert the color value of each pixel in an
pub const LUMINANCE_TO_ALPHA_FILTER: ColorFilterMatrix = ColorFilterMatrix {
  matrix: [
    0., 0., 0., 0., // red
    0., 0., 0., 0., // green
    0., 0., 0., 0., // blue
    0.2125, 0.7154, 0.0721, 0., // alpha
  ],
  base_color: None,
};

/// saturate_filter
///
/// Creates a color filter that changes the saturation of an element's color
/// palette, altering its overall color tone.
/// Parameters:
///   - level: The saturation_level parameter accepts values between 0.0 and 1.0 (float).
///       - level < 0.5 desaturate colors (grayscale effect).
///       - level > 0.5 saturate colors (vibrant effect).
///       - and level = 1.0 maintains original saturation.
#[rustfmt::skip]
pub fn saturate_filter(level: f32) -> ColorFilterMatrix {
  ColorFilterMatrix {
    matrix: [
      0.213 + 0.787 * level, 0.715 - 0.715 * level, 0.072 - 0.072 * level, 0.,  // red
      0.213 - 0.213 * level, 0.715 + 0.285 * level, 0.072 - 0.072 * level, 0.,  // green
      0.213 - 0.213 * level, 0.715 - 0.715 * level, 0.072 + 0.928 * level, 0.,  // blue
      0., 0., 0., 1.,  //alpha
    ],
    base_color: None,
  }
}

/// grayscale_filter
/// 
/// Creates a color filter that converts the color value of each pixel in an image to grayscale.
/// Parameters:
///   - amount: The amount parameter accepts values between 0.0 and 1. value 1.0 means full grayscale, while 0.0 means no change.
#[rustfmt::skip]
pub fn grayscale_filter(amount: f32) -> ColorFilterMatrix {
  let t = amount.clamp(0.0,1.0);
  let (r, g, b) = (0.2126, 0.7152, 0.0722);
  ColorFilterMatrix {
    matrix: [
      1.0 - t + t * r,   t * g,             t * b,             0.0, // red
      t * r,             1.0 - t + t * g,   t * b,             0.0, // green
      t * r,             t * g,             1.0 - t + t * b,   0.0, // blue
      0.0,               0.0,               0.0,               1.0, // alpha
    ],
  base_color: None,
  }
}

/// opacity_filter
/// 
/// Creates a color filter that changes the opacity of the color.
/// Parameters:
///   - amount: The amount parameter accepts values between 0.0 and 1. value 1.0 means no changed, while 0.0 means transparent.
#[rustfmt::skip]
pub fn opacity_filter(amount: f32) -> ColorFilterMatrix {
  let v = amount.clamp(0.0, 1.0);
  ColorFilterMatrix {
    matrix: [
      1.0, 0.0, 0.0, 0.0,
      0.0, 1.0, 0.0, 0.0,
      0.0, 0.0, 1.0, 0.0,
      0.0, 0.0, 0.0, v
    ],
    base_color: None,
  }
}

/// contrast_filter
///
/// Creates a color filter that adjusts the contrast of an element's color
/// palette, altering the distinction between light and dark areas.
/// Parameters:
///   - amount: The contrast adjustment parameter accepts values between 0.0 and
///     1.0 (float).
///       - amount = 0.0: Minimum contrast (all colors become neutral gray 0.5)
///       - 0.0 < amount < 1.0: Gradual contrast adjustment
///       - amount = 1.0: Maximum contrast (original colors preserved)
///
/// Visual effect:
///   - Lower values reduce contrast, flattening color differences
///   - Higher values enhance contrast, intensifying color separation
pub fn contrast_filter(amount: f32) -> ColorFilterMatrix {
  let c = amount.clamp(0.0, 1.0);
  let offset = 0.5 * (1.0 - c);

  ColorFilterMatrix {
    matrix: [
      c, 0.0, 0.0, 0.0, // R 行
      0.0, c, 0.0, 0.0, // G 行
      0.0, 0.0, c, 0.0, // B 行
      0.0, 0.0, 0.0, 1.0, // A 行
    ],
    base_color: Some(Color::from_f32_rgba(offset, offset, offset, 0.0)),
  }
}

/// brightness_filter
/// 
/// Creates a color filter that adjusts the brightness of an element's color palette.
/// Parameters:
///    - amount: The amount parameter accepts values between 0.0 and INFINITY.
///        - value == 1.0, no change, 
///        - values < 1.0, darken the colors,
///        - values > 1.0, brighten the colors.
#[rustfmt::skip]
pub fn brightness_filter(amount: f32) -> ColorFilterMatrix {
    let t = (amount - 1.0).max(-1.0); 
    ColorFilterMatrix {
      matrix: [
        1.,   0.0,  0.0,  0.0, 
        0.0,  1.,   0.0,  0.0, 
        0.0,  0.0,  1.,    0.0, 
        0.0,  0.0,  0.0,  1.0,
      ],
      base_color: Some(Color::from_f32_rgba(t, t, t, 0.0)),
    }
}

/// invert_filter
/// 
/// Creates a color filter that inverts the color value of each pixel in an image.
/// Parameters:
///    - amount: The amount parameter accepts values between 0.0 and 1.0. 1.0 means full invert, while 0.0 means no change.
#[rustfmt::skip]
pub fn invert_filter(amount: f32) -> ColorFilterMatrix {
    let i = amount.clamp(0.0, 1.0);
    ColorFilterMatrix {
      matrix: [
        1. - 2. * i,    0.0,             0.0,             0.0,
        0.0,            1. - 2. * i,     0.0,             0.0,
        0.0,            0.0,             1. - 2. * i,     0.0,
        0.0,            0.0,             0.0,             1.0,
      ],
      base_color: Some(Color::from_f32_rgba(i, i, i, 0.)),
    }
}

/// hue_rotate_filter
///
/// Creates a color filter that rotates the hue of an element's color palette by
/// a specified angle, altering its overall color tone.
/// Parameters:
///   - rad: Value in radians (deg), positive values rotate clockwise, negative
///     values rotate counterclockwise.
#[rustfmt::skip]
pub fn hue_rotate_filter(rad: f32) -> ColorFilterMatrix {
  ColorFilterMatrix {
    matrix: [
      0.213 + rad.cos() * 0.787 - rad.sin() * 0.213, 0.715 - rad.cos() * 0.715 - rad.sin() * 0.715, 0.072 - rad.cos() * 0.072 + rad.sin() * 0.928, 0.,
      0.213 - rad.cos() * 0.213 + rad.sin() * 0.143, 0.715 + rad.cos() * 0.285 + rad.sin() * 0.14,  0.072 - rad.cos() * 0.072 - rad.sin() * 0.283, 0.,
      0.213 - rad.cos() * 0.213 - rad.sin() * 0.787, 0.715 - rad.cos() * 0.715 + rad.sin() * 0.715, 0.072 + rad.cos() * 0.928 + rad.sin() * 0.072, 0.,
      0.,0.,0.,1.,
    ],
    base_color: None 
  }
}

/// INVERT_FILTER
///
/// INVERT_FILTER will invert the color value of each pixel in an image, that
/// is, the original bright color becomes dark, and the dark color becomes
/// bright.
pub const INVERT_FILTER: ColorFilterMatrix = ColorFilterMatrix {
  matrix: [
    -1., 0., 0., 0., // red
    0., -1., 0., 0., // green
    0., 0., -1., 0., // blue
    0., 0., 0., 1., // alpha
  ],
  base_color: Some(Color::from_f32_rgba(1., 1., 1., 0.)),
};

/// This widget applies a [`ColorFilterMatrix`] to the child widget.
#[derive(Declare)]
pub struct ColorFilter {
  pub filter: ColorFilterMatrix,
}

impl_compose_child_for_wrap_render!(ColorFilter);

impl WrapRender for ColorFilter {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.painter().apply_color_matrix(self.filter);
    host.paint(ctx);
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  #[cfg(feature = "png")]
  widget_image_tests!(
    color_filter,
    WidgetTester::new(fn_widget! {
      let img = Resource::new(PixelImage::from_png(include_bytes!("../../../gpu/imgs/leaves.png")));
      let svg = Svg::parse_from_bytes(
        include_bytes!("../../../static/logo.svg"), true, false,
      ).unwrap();
      @ColorFilter {
        filter: GRAYSCALE_FILTER,
        @ Column {
          @Icon { @{ svg } }
          @ { img }
        }
      }
    })
    .with_wnd_size(Size::new(260., 160.))
    .with_comparison(0.00006)
  );
}
