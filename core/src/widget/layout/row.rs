use super::flex::*;
use crate::prelude::*;

#[derive(Default, MultiChildWidget)]
pub struct Row(Flex);

impl RenderWidget for Row {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.0.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.0.only_sized_by_parent() }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
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

impl RowBuilder {
  pub fn into_reverse(v: bool) -> bool { v }
  pub fn into_wrap(v: bool) -> bool { v }
  pub fn into_cross_align(v: CrossAxisAlign) -> CrossAxisAlign { v }
  pub fn into_main_align(v: MainAxisAlign) -> MainAxisAlign { v }
}
