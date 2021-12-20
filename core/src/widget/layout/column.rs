use super::flex::*;
use crate::prelude::*;

#[derive(RenderWidget, MultiChildWidget)]
pub struct Column(#[proxy] Flex);

impl Default for Column {
  fn default() -> Self { ColumnBuilder::default().build() }
}

impl IntoStateful for Column {
  type S = StatefulFlex;
  fn into_stateful(self) -> Self::S { self.0.into_stateful() }
}

// declare macro  support.
#[derive(Default)]
pub struct ColumnBuilder {
  pub reverse: bool,
  pub wrap: bool,
  pub cross_align: CrossAxisAlign,
  pub main_align: MainAxisAlign,
}

impl Declare for Column {
  type Builder = ColumnBuilder;
}

impl DeclareBuilder for ColumnBuilder {
  type Target = Column;

  #[inline]
  fn build(self) -> Self::Target {
    let Self {
      reverse,
      wrap,
      cross_align,
      main_align,
    } = self;
    Column(
      FlexBuilder {
        reverse,
        wrap,
        direction: Direction::Vertical,
        cross_align,
        main_align,
      }
      .build(),
    )
  }
}
