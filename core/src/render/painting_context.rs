use super::render_tree::*;
use canvas::Canvas;

pub struct PaintingContext<'a> {
  painting_node: RenderId,
  tree: &'a RenderTree,
  cavnas: &'a mut Canvas,
}

impl<'a, 'b> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(
    frame: &'a mut Canvas,
    painting_node: RenderId,
    tree: &'a RenderTree,
  ) -> Self {
    Self {
      cavnas: frame,
      tree,
      painting_node,
    }
  }

  pub fn paint_child(&mut self, child_id: RenderId) {
    let ctx = PaintingContext {
      cavnas: self.cavnas,
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
