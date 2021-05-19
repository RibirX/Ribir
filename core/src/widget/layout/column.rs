use super::flex::*;
use crate::prelude::*;

#[derive(Widget, RenderWidget)]
pub struct Column(#[proxy] Flex);

impl Column {
  #[inline]
  pub fn push<W: Widget>(self, child: W) -> Self { Column(self.0.push(child)) }

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

impl std::iter::FromIterator<BoxWidget> for Column {
  fn from_iter<T: IntoIterator<Item = BoxWidget>>(iter: T) -> Self {
    Self(Flex::from_iter(iter).with_direction(Direction::Vertical))
  }
}
