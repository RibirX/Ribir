use slab_tree::*;
use std::fmt::Debug;

pub struct RenderCtx {
  tree: Tree<Box<dyn RenderObject>>,
  current_id: NodeId,
}
pub trait RenderObject: Debug {
  fn paint(&self);
  fn perform_layout(&mut self, ctx: RenderCtx);
}
