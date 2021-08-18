use super::flex::*;
use crate::prelude::*;

#[derive(RenderWidget, MultiChildWidget, AttachAttr)]
pub struct Row(#[proxy] Flex);

impl Row {
  #[inline]
  pub fn with_reverse(self, reverse: bool) -> Self { Self(self.0.with_reverse(reverse)) }

  #[inline]
  pub fn with_wrap(self, wrap: bool) -> Self { Self(self.0.with_wrap(wrap)) }

  #[inline]
  pub fn with_cross_align(self, align: CrossAxisAlign) -> Self {
    Self(self.0.with_cross_align(align))
  }

  #[inline]
  pub fn with_main_align(self, align: MainAxisAlign) -> Self { Self(self.0.with_main_align(align)) }

  #[inline]
  pub fn get_cross_align(&self) -> CrossAxisAlign { self.0.cross_align }
}

impl Default for Row {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Horizontal)) }
}

impl IntoStateful for Row {
  type S = StatefulFlex;
  fn into_stateful(self) -> Self::S { self.0.into_stateful() }
}
