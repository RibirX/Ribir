use slab_tree::*;

pub struct RenderCtx {
  tree: Tree<Box<dyn RenderObject>>,
  current_id: NodeId,
}
pub trait RenderObject {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String;
  fn paint(&self);
  fn perform_layout(&mut self, ctx: RenderCtx);
}

#[cfg(debug_assertions)]
use std::fmt::{Debug, Formatter, Result};
impl Debug for Box<dyn RenderObject> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str(&self.to_str()) }
}
