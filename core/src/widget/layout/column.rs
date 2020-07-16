use super::flex::*;
use crate::prelude::*;

#[derive(Debug)]
pub struct Column(Flex);

impl Column {
  #[inline]
  pub fn push<W: Widget>(&mut self, child: W) -> &mut Self {
    self.0.push(child);
    self
  }

  #[inline]
  pub fn with_reverse(self, reverse: bool) -> Self { Self(self.0.with_reverse(reverse)) }

  #[inline]
  pub fn with_wrap(self, wrap: bool) -> Self { Self(self.0.with_wrap(wrap)) }
}

impl Default for Column {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Vertical)) }
}

impl std::iter::FromIterator<BoxWidget> for Column {
  fn from_iter<T: IntoIterator<Item = BoxWidget>>(iter: T) -> Self {
    Self(Flex::from_iter(iter).with_direction(Direction::Vertical))
  }
}

inherit_widget!(Column, 0);
