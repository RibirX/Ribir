use wrap_render::WrapRender;

use crate::prelude::*;

/// A widget that reduces the available content area by applying padding
/// around its child.
///
/// This is a built-in `FatObj` field. Setting the `padding` field attaches a
/// `Padding` wrapper that offsets and constrains its child by the given
/// `EdgeInsets`.
///
/// # Example
///
/// Show text inside a container with 10px padding.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   padding: EdgeInsets::all(10.),
///   background: Color::BLUE,
///   @Text { text: "Padding example" }
/// };
/// ```
#[derive(Default)]
pub struct Padding {
  pub padding: EdgeInsets,
}

impl Declare for Padding {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(Padding);

impl WrapRender for Padding {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let thickness = self.padding.thickness();

    let min = (clamp.min - thickness).max(ZERO_SIZE);
    let max = (clamp.max - thickness).max(ZERO_SIZE);
    // Shrink the clamp of child.
    let child_clamp = BoxClamp { min, max };
    let size = host.perform_layout(child_clamp, ctx);
    clamp.clamp(size + thickness)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.content_only_transform_apply(&Transform::translation(self.padding.left, self.padding.top));
    host.paint(ctx);
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    host
      .visual_box(ctx)
      .map_or(Some(Rect::from_size(ctx.box_size()?)), |mut rect| {
        rect.size += self.padding.thickness();
        Some(rect)
      })
  }

  /// Returns the transform to apply to the child widget.
  /// The padding is applied when mapping the child's coordinates(content area).
  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    let padding_matrix = Transform::translation(self.padding.left, self.padding.top);

    let ts = host
      .get_transform()
      .map_or(padding_matrix, |h| padding_matrix.then(&h));

    Some(ts)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl Padding {
  #[inline]
  pub fn new(padding: EdgeInsets) -> Self { Self { padding } }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @MockMulti {
        padding: EdgeInsets::only_left(1.),
        @MockBox {
           size: Size::new(100., 100.),
        }
      }
    }),
    // MockMulti widget
    LayoutCase::default()
      .with_size(Size::new(101., 100.))
      .with_x(0.),
    // The MockBox
    // padding does not update the children's position, but it transforms during painting and hit
    // testing.
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 100.))
  );

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn fix_padding_draw() {
    crate::reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(text! {
        padding: EdgeInsets::all(10.),
        background: Color::GREEN,
        text: "Hello, Ribir!"
      })
      .with_wnd_size(Size::new(128., 48.))
      .with_comparison(0.000025),
      "padding_draw"
    );
  }
}
