//! Filter widgets for applying visual effects.

use ribir_painter::Filter;

use crate::{prelude::*, wrap_render::WrapRender};

/// A widget that applies a filter to its content.
///
/// This is a builtin field of FatObj. You can simply set the `filter` field
/// to attach a FilterWidget to the host widget.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Container {
///     size: Size::new(100., 100.),
///     @Text {
///       filter: Filter::blur(20.),
///       text: "Hello, Ribir!",
///     }
///   }
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
    filter,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(200., 40.),
        @Text {
          filter: Filter::blur(2.),
          font_size: 28.,
          text_line_height: 32.,
          text: "Hello, Ribir!",
        }
      }
    })
    .with_comparison(0.00006)
    .with_wnd_size(Size::new(200., 40.))
  );
}
