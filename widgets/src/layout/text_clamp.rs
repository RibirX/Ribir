use ribir_core::{impl_compose_child_for_wrap_render, prelude::*, wrap_render::WrapRender};

/// A widget that constrains its children to a certain size, the size as
/// characters laid out to rows * columns based on text style metrics.
#[derive(Declare)]
pub struct TextClamp {
  /// If rows is Some(rows), the height of the child will be constrained to
  /// rows * text_style.line_height
  /// Default is None, no additional constraints on height will be applied
  #[declare(default)]
  pub rows: Option<f32>,

  /// If columns is Some(cols), the width of the child will be constrained to
  /// columns * text_style.font_size.
  /// Default is None, no additional constraints on width will be applied
  #[declare(default)]
  pub cols: Option<f32>,
}

impl_compose_child_for_wrap_render!(TextClamp);

impl WrapRender for TextClamp {
  fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let text_style = Provider::of::<TextStyle>(ctx).unwrap();
    if let Some(rows) = self.rows {
      let height = rows * text_style.line_height;
      clamp = clamp.with_fixed_height(height.clamp(clamp.min.height, clamp.max.height));
    }
    if let Some(cols) = self.cols {
      let width = cols * text_style.font_size;
      clamp = clamp.with_fixed_width(width.clamp(clamp.min.width, clamp.max.width));
    }

    host.perform_layout(clamp, ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  const WND_SIZE: Size = Size::new(200., 200.);

  widget_layout_test!(
    text_clamp_row_cols,
    WidgetTester::new(fn_widget! {
      @TextClamp {
        rows: Some(1.5),
        cols: Some(15.),
        font_size: 12.,
        text_line_height: 16.,
        @Container {
          size: WND_SIZE
        }
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(Size::new(180., 24.))
  );

  widget_layout_test!(
    text_clamp_rows,
    WidgetTester::new(fn_widget! {
      @TextClamp {
        rows: Some(1.5),
        text_line_height: 16.,
        @Container {
          size: WND_SIZE
        }
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(Size::new(200., 24.))
  );

  widget_layout_test!(
    text_clamp_cols,
    WidgetTester::new(fn_widget! {
      @TextClamp {
        cols: Some(15.5),
        font_size: 12.,
        @Container {
          size: WND_SIZE
        }
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(Size::new(186., 200.))
  );
}
