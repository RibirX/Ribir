use ribir_painter::color::ColorFilterMatrix;

use crate::prelude::*;

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
#[derive(Declare, SingleChild)]
pub struct ColorFilter {
  pub filter: ColorFilterMatrix,
}

impl Render for ColorFilter {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_color_matrix(self.filter); }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  #[cfg(feature = "png")]
  widget_image_tests!(
    default_text,
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
