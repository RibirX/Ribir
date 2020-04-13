use crate::render_ctx::RenderCtx;
use indextree::*;

bitflags! {
    pub struct LayoutConstraints: u8 {
        const DECIDED_BY_SELF = 0;
        const EFFECTED_BY_PARENT = 2;
        const EFFECTED_BY_CHILDREN = 2;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Size {
  pub width: i32,
  pub height: i32,
}

#[derive(Debug, Clone)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}

pub trait RenderObjectBox {
  fn bound(&self) -> Option<Size>;
  fn get_constraints(&self) -> LayoutConstraints;

  fn layout_sink(&mut self, self_id: NodeId, ctx: &mut RenderCtx);
  fn layout_bubble(&mut self, self_id: NodeId, ctx: &mut RenderCtx);

  fn mark_dirty(&mut self);
  fn is_dirty(&self) -> bool;
}
