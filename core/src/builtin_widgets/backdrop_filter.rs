use crate::{
  prelude::{color::ColorFilterMatrix, *},
  wrap_render::WrapRender,
};

/// A widget that applies a backdrop filter to background content.
#[derive(Default, Clone)]
pub struct BackdropFilter {
  pub filters: Vec<Vec<FilterType>>,
}

impl Declare for BackdropFilter {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for BackdropFilter {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    host.paint(ctx);

    if !size.is_empty() {
      let path = Path::rect(&Rect::from_size(size)).into();
      let filters = self
        .filters
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
      if filters.is_empty() {
        return;
      }

      ctx.painter().filters(path, filters);
    }
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}

impl BackdropFilter {
  /// Creates a blur filter with the specified radius.
  /// Note that the radius should be less equal to 30.
  pub fn blur_filter(radius: usize) -> Vec<FilterType> {
    if radius == 0 {
      return vec![];
    }

    if 30 < radius {
      log::warn!("BackdropFilter::blur_filter radius the radius should be less equal to 30");
    }
    let radius = radius.min(30);
    let kernel = gaussian_kernel(radius, radius as f32 / 2.);
    vec![
      FilterType::Convolution(FlattenMatrix {
        width: kernel.len(),
        height: 1,
        matrix: kernel.clone(),
      }),
      FilterType::Convolution(FlattenMatrix { width: 1, height: kernel.len(), matrix: kernel }),
    ]
  }

  /// Creates a color filter with the specified color matrix.
  pub fn color_filter(filter: ColorFilterMatrix) -> Vec<FilterType> {
    vec![FilterType::Color(filter)]
  }
}

/// Generates a Gaussian 1 dimension kernel with the specified radius and sigma.
/// The sigma must be greater than 0.
pub fn gaussian_kernel(radius: usize, sigma: f32) -> Vec<f32> {
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

  kernel.iter_mut().for_each(|w| *w /= sum);
  kernel
}

impl_compose_child_for_wrap_render!(BackdropFilter);

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  #[cfg(feature = "png")]
  widget_image_tests!(
    backdrop_filter,
    WidgetTester::new(fn_widget! {
      let img = Resource::new(PixelImage::from_png(include_bytes!("../../../gpu/imgs/leaves.png")));

      @Stack {
        @ { img }
        @Container {
          anchor: Anchor::left_top(20., 20.),
          size: Size::new(80., 80.),
          backdrop_filter: [
            BackdropFilter::color_filter(GRAYSCALE_FILTER),
            BackdropFilter::blur_filter(3)
          ],
        }
      }
    })
    .with_wnd_size(Size::new(260., 160.))
    .with_comparison(0.00006)
  );
}
