use crate::prelude::*;

/// A simple container widget with a fixed size for its child.
///
/// # Example
///
/// Place text inside a 100x100 container.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   width: 100.,
///   height: 100.,
///   background: Color::BLUE,
///   @Text { text: "Hello" }
/// };
/// ```
#[derive(Declare, SingleChild, Clone)]
pub struct Container {
  #[declare(default = Measure::Percent(1.))]
  pub width: Measure,
  #[declare(default = Measure::Percent(1.))]
  pub height: Measure,
}

impl Render for Container {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let width = self
      .width
      .into_pixel(clamp.max.width)
      .clamp(clamp.min.width, clamp.max.width);
    let height = self
      .height
      .into_pixel(clamp.max.height)
      .clamp(clamp.min.height, clamp.max.height);

    let size = Size::new(width, height);
    let child_clamp = BoxClamp::max_size(size);

    ctx.perform_single_child_layout(child_clamp);
    size
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { false }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  const TEST_SIZE: Size = Size::new(100., 100.);

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @Container {
        width: 100.,
        height: 100.,
      }
    }),
    LayoutCase::default().with_size(TEST_SIZE)
  );
}
