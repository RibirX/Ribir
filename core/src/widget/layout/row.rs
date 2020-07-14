use super::flex::*;
use crate::prelude::*;

#[derive(Debug)]
pub struct Row(Flex);

impl Row {
  pub fn from_iter(children: impl Iterator<Item = BoxWidget>) -> Self {
    Self(Flex::from_iter(children).with_direction(Direction::Horizontal))
  }

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

impl Default for Row {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Horizontal)) }
}

impl Widget for Row {
  #[inline]
  fn classify(&self) -> WidgetClassify { self.0.classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.0.classify_mut() }
}
