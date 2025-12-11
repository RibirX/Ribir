use crate::{prelude::*, wrap_render::WrapRender};

/// A widget that applies a backdrop filter to background content.
#[derive(Default, Clone)]
pub struct BackdropFilter {
  pub filter: Filter,
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

    if !size.is_empty() && !self.filter.is_empty() {
      let path = Path::rect(&Rect::from_size(size)).into();
      ctx
        .painter()
        .filter_path(path, self.filter.clone());
    }
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
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
          backdrop_filter: Filter::grayscale(1.).with(Filter::blur(3.)),
        }
      }
    })
    .with_wnd_size(Size::new(260., 160.))
    .with_comparison(0.00006)
  );
}
