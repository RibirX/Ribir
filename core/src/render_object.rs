use slab_tree::*;

pub struct RenderCtx {
  tree: Tree<Box<dyn RenderObject>>,
  current_id: NodeId,
}
pub trait RenderObject {
  fn paint(&self);
  fn perform_layout(&mut self, ctx: RenderCtx);
}
