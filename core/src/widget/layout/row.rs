use super::flex::*;
use crate::prelude::*;

#[derive(RenderWidget, MultiChildWidget)]
pub struct Row(#[proxy] Flex);

impl Default for Row {
  fn default() -> Self { Self(Flex::default().with_direction(Direction::Horizontal)) }
}

impl IntoStateful for Row {
  type S = StatefulFlex;
  fn into_stateful(self) -> Self::S { self.0.into_stateful() }
}

// declare macro  support.
#[derive(Default)]
pub struct RowBuilder {
  pub reverse: bool,
  pub wrap: bool,
  pub cross_align: CrossAxisAlign,
  pub main_align: MainAxisAlign,
}

impl Declare for Row {
  type Builder = RowBuilder;
}

impl DeclareBuilder for RowBuilder {
  type Target = Row;

  #[inline]
  fn build(self) -> Self::Target {
    let Self {
      reverse,
      wrap,
      cross_align,
      main_align,
    } = self;
    Row(
      FlexBuilder {
        reverse,
        wrap,
        direction: Direction::Horizontal,
        cross_align,
        main_align,
      }
      .build(),
    )
  }
}
