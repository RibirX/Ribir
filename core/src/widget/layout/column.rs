use super::flex::*;
use crate::prelude::*;

#[derive(Default, MultiChildWidget, Declare)]
pub struct Column {
  pub reverse: bool,
  pub wrap: bool,
  pub cross_align: CrossAxisAlign,
  pub main_align: MainAxisAlign,
}

impl RenderWidget for Column {
  type RO = Flex;

  fn create_render_object(&self) -> Self::RO {
    Flex {
      reverse: self.reverse,
      wrap: self.wrap,
      direction: Direction::Vertical,
      cross_align: self.cross_align,
      main_align: self.main_align,
    }
  }

  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    let mut need_layout = false;
    if self.reverse != object.reverse {
      object.reverse = self.reverse;
      need_layout = true;
    }
    if self.wrap != object.wrap {
      object.wrap = self.wrap;
      need_layout = true;
    }
    if self.cross_align != object.cross_align {
      object.cross_align = self.cross_align;
      need_layout = true;
    }
    if self.main_align != object.main_align {
      object.main_align = self.main_align;
      need_layout = true;
    }
    if need_layout {
      ctx.mark_needs_layout()
    }
  }
}
