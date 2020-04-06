use crate::render_ctx::RenderCtx;
use indextree::*;

#[derive(PartialEq)]
pub enum LayoutConstraints {
  DecidedBySelf,
  EffectedByParent,
  EffectedByChildren,
  EffectedByBoth,
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

  fn layout_sink(&mut self, ctx: &mut RenderCtx, self_id: NodeId);
  fn layout_bubble(&mut self, ctx: &mut RenderCtx, self_id: NodeId);

  fn mark_dirty(&mut self);
  fn is_dirty(&self) -> bool;
}
