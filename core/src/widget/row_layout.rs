use crate::prelude::*;
use crate::render_ctx::RenderCtx;
use crate::render_object_box::{
  LayoutConstraints, Position, RenderObjectBox, Size,
};
use indextree::*;
///  a stupid implement for develope the framework.

pub struct Row<'a> {
  pub children: Vec<Widget<'a>>,
}

impl<'a> MultiChildWidget<'a> for Row<'a> {
  fn split(self: Box<Self>) -> (Box<dyn RenderWidget + 'a>, Vec<Widget<'a>>) {
    (Box::new(RenderRow {}), self.children)
  }
}

#[derive(Debug)]
struct RowRenderObject {
  inner_layout: Vec<(Position, Size)>,
  size: Option<Size>,
}

impl RenderObject for RowRenderObject {
  fn paint(&self) {}
  // fn perform_layout(&mut self, node_id: NodeId, _ctx: &mut RenderCtx);
  fn to_render_box(&self) -> Option<&dyn RenderObjectBox> { Some(self) }
  fn to_render_box_mut(&mut self) -> Option<&mut dyn RenderObjectBox> {
    Some(self)
  }
}

impl RenderObjectBox for RowRenderObject {
  fn bound(&self) -> Option<Size> { return self.size.clone(); }
  fn get_constraints(&self) -> LayoutConstraints {
    LayoutConstraints::EFFECTED_BY_CHILDREN
  }

  fn layout_sink(&mut self, _self_id: NodeId, _ctx: &mut RenderCtx) {}
  fn layout_bubble(&mut self, self_id: NodeId, ctx: &mut RenderCtx) {
    let mut x = 0 as i32;
    let y = 0;

    let mut ids = vec![];
    ctx.collect_children_box(self_id, &mut ids);
    ids.reverse();
    for id in ids {
      let node = ctx.tree.get_mut(id).unwrap();
      let render_box = node.get_mut().to_render_box().unwrap();
      let bound = render_box.bound().unwrap();
      self
        .inner_layout
        .push((Position { x: x, y: y }, bound.clone()));
      x += bound.width;
    }
    self.size = Some(Size {
      width: x,
      height: 1,
    });
  }
  fn mark_dirty(&mut self) {
    self.size = None;
    self.inner_layout.clear();
  }
  fn is_dirty(&self) -> bool { return self.size.is_none(); }
}

#[derive(Debug)]
struct RenderRow {}

impl RenderWidget for RenderRow {
  fn create_render_object(&self) -> Box<dyn RenderObject + Send + Sync> {
    Box::new(RowRenderObject {
      inner_layout: vec![],
      size: None,
    })
  }
}
