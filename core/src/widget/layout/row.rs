use super::flex::*;
use crate::prelude::*;

#[derive(Debug)]
pub struct Row(Flex);

impl Row {
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

impl std::iter::FromIterator<BoxWidget> for Row {
  fn from_iter<T: IntoIterator<Item = BoxWidget>>(iter: T) -> Self {
    Self(Flex::from_iter(iter).with_direction(Direction::Horizontal))
  }
}

impl Default for Row {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Horizontal)) }
}

inherit_widget!(Row, 0);
