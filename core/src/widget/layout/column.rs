use super::flex::*;
use crate::prelude::*;

#[derive(RenderWidget, MultiChildWidget, AttachAttr)]
pub struct Column(#[proxy] Flex);

impl Column {
  #[inline]
  pub fn with_reverse(self, reverse: bool) -> Self { Self(self.0.with_reverse(reverse)) }

  #[inline]
  pub fn with_wrap(self, wrap: bool) -> Self { Self(self.0.with_wrap(wrap)) }

  #[inline]
  pub fn with_cross_align(self, align: CrossAxisAlign) -> Self {
    Self(self.0.with_cross_align(align))
  }
}

impl Default for Column {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Vertical)) }
}

impl IntoStateful for Column {
  type S = StatefulFlex;
  fn into_stateful(self) -> Self::S { self.0.into_stateful() }
}
