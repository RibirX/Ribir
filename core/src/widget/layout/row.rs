use super::flex::*;
use crate::{impl_query_self_only, prelude::*};

// todo: give a alias for `CrossAxisAlign` and `MainAxisAlign`

#[derive(Default, MultiChild, Declare, Clone)]
pub struct Row {
  #[declare(default)]
  pub reverse: bool,
  #[declare(default)]
  pub wrap: bool,
  #[declare(default = "CrossAxisAlign::Center")]
  pub v_align: CrossAxisAlign,
  #[declare(default)]
  pub h_align: MainAxisAlign,
}

impl Render for Row {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let Self { reverse, wrap, v_align, h_align } = self.clone();

    Flex {
      reverse,
      wrap,
      direction: Direction::Horizontal,
      cross_align: v_align,
      main_align: h_align,
    }
    .perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Row {
  impl_query_self_only!();
}
