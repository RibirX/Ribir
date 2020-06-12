use super::flex::{Axis, FlexContainer};
use crate::render::default_box_impl;

use crate::prelude::*;
use crate::render::render_ctx::RenderCtx;
use crate::render::render_tree::*;
use crate::render::LayoutConstraints;

///  a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Row(pub Vec<Box<dyn Widget>>);

impl Widget for Row {
  multi_child_widget_base_impl!();
}

#[derive(Debug)]
pub struct RowRender {
  pub row: FlexContainer,
}

impl RenderWidget for Row {
  type RO = RowRender;
  fn create_render_object(&self) -> Self::RO {
    RowRender {
      row: FlexContainer::new(Axis::Horizontal, LayoutConstraints::EFFECTED_BY_CHILDREN),
    }
  }
}

impl MultiChildWidget for Row {
  fn take_children(&mut self) -> Vec<Box<dyn Widget>> { std::mem::take(&mut self.0) }
}

impl RenderObject<Row> for RowRender {
  fn update(&mut self, _owner_widget: &Row) {}

  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) { self.row.flex_layout(id, ctx); }
  #[inline]
  fn paint<'b>(&'b self, _ctx: &mut PaintingContext<'b>) {}
  #[inline]
  fn child_offset(&self, _idx: usize) -> Option<Point> { None }
  default_box_impl!({ row.bound });
}
