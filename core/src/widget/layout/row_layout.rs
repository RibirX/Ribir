use super::flex::{Axis, FlexContainer};
use crate::render::default_box_impl;

use crate::prelude::*;
use crate::render::render_ctx::RenderCtx;
use crate::render::render_tree::*;
use crate::render::LayoutConstraints;

///  a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Row<'a>(pub Vec<Box<dyn Widget + 'a>>);

impl<'a> Widget for Row<'a> {
  multi_child_widget_base_impl!();
}

#[derive(Debug)]
pub struct RowRender {
  pub row: FlexContainer,
}

impl<'a> RenderWidget for Row<'a> {
  type RO = RowRender;
  fn create_render_object(&self) -> Self::RO {
    RowRender {
      row: FlexContainer::new(Axis::Horizontal, LayoutConstraints::EFFECTED_BY_CHILDREN),
    }
  }
}

impl<'a> MultiChildWidget for Row<'a> {
  fn take_children<'b>(&mut self) -> Vec<Box<dyn Widget + 'a>>
  where
    Self: 'b,
  {
    std::mem::take(&mut self.0)
  }
}

impl<'a> RenderObject<Row<'a>> for RowRender {
  fn update(&mut self, _owner_widget: &Row<'a>) {}

  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) { self.row.flex_layout(id, ctx); }
  fn paint<'b>(&'b self, ctx: &mut PaintingContext<'b>) {}
  fn child_offset(&self, idx: usize) -> Option<Point> { None }
  default_box_impl!({ row.bound });
}
