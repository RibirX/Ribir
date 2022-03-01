use super::flex::*;
use crate::prelude::*;

// todo: give a alias for `CrossAxisAlign` and `MainAxisAlign`

#[derive(Default, MultiChildWidget, Declare, Clone)]
pub struct Column {
  #[declare(default)]
  pub reverse: bool,
  #[declare(default)]
  pub wrap: bool,
  #[declare(default)]
  pub h_align: CrossAxisAlign,
  #[declare(default)]
  pub v_align: MainAxisAlign,
}

impl RenderWidget for Column {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let Self { reverse, wrap, h_align, v_align } = self.clone();

    Flex {
      reverse,
      wrap,
      direction: Direction::Vertical,
      cross_align: h_align,
      main_align: v_align,
    }
    .perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}
