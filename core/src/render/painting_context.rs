use super::render_tree::*;
use canvas::Frame;

pub struct PaintingContext<'a, 'b> {
  painting_node: RenderId,
  tree: &'a RenderTree,
  frame: &'a mut Frame<'b>,
}

impl<'a, 'b> PaintingContext<'a, 'b> {
  #[inline]
  pub(crate) fn new(
    frame: &'a mut Frame<'b>,
    painting_node: RenderId,
    tree: &'a RenderTree,
  ) -> Self {
    Self {
      frame,
      tree,
      painting_node,
    }
  }

  pub fn paint_child(&mut self, child_id: RenderId) {
    let ctx = PaintingContext {
      frame: &mut self.frame,
      tree: self.tree,
      painting_node: child_id,
    };

    child_id
      .get(&self.tree)
      .expect("Child must exists!")
      .paint(ctx);
  }

  /// Returns an iterator of references to the painting render object's
  /// children.
  #[inline]
  pub fn children(&self) -> impl Iterator<Item = RenderId> + 'a {
    self.painting_node.children(&self.tree)
  }
}
