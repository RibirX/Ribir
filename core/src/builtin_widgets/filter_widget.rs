//! Filter widgets for applying visual effects.

use ribir_painter::Filter;

use crate::{prelude::*, wrap_render::WrapRender};

/// A wrapper that applies a painter `Filter` to its content subtree.
///
/// This is a built-in `FatObj` field. Setting the `filter` field attaches a
/// `FilterWidget` to the host, causing subsequent painting to run through the
/// configured filter pipeline.
///
/// # Example
///
/// Apply a blur filter to the text content.
///
/// ```rust
/// use ribir::prelude::*;
///
/// text! {
///   filter: Filter::blur(20.),
///   text: "Hello, Ribir!",
/// };
/// ```
#[derive(Default, Clone)]
pub struct FilterWidget {
  pub filter: Filter,
}

impl Declare for FilterWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for FilterWidget {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.filter.is_empty() {
      host.paint(ctx);
      return;
    }

    ctx.painter().filter(self.filter.clone());
    host.paint(ctx);
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("filter") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({ "empty": self.filter.is_empty() }))
  }
}

impl_compose_child_for_wrap_render!(FilterWidget);

/// Macro for creating a color filter widget.
#[macro_export]
macro_rules! filter {
  ($($t: tt)*) => {
    fn_widget! {
      let mut obj = FatObj::<()>::default();
      @(obj) { $($t)* }
    }
  };
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  widget_image_tests!(
    filter_drop_shadow,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(600., 200.),
        // filter apply to the subtree
        filter: Filter::drop_shadow((100., 10.), 10., Color::BLACK.with_alpha(0.5)),
        @Container {
          x: AnchorX::center(),
          y: AnchorY::center(),
          size: Size::new(100., 100.),
          background: Color::YELLOW,
        }
      }
    })
    .with_comparison(0.00006)
    .with_wnd_size(Size::new(600., 200.))
  );
}
