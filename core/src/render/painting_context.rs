use super::{canvas::CanvasRenderingContext2D, render_tree::*};

pub struct PaintingContext<'a> {
  painting_node: RenderId,
  tree: &'a RenderTree,
  canvas: &'a mut CanvasRenderingContext2D,
}

impl<'a> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(
    canvas: &'a mut CanvasRenderingContext2D,
    painting_node: RenderId,
    tree: &'a RenderTree,
  ) -> Self {
    Self {
      canvas,
      tree,
      painting_node,
    }
  }

  #[inline]
  pub fn canvas(&mut self) -> &mut CanvasRenderingContext2D { &mut self.canvas }

  pub fn paint_child(&mut self, child_id: RenderId) {
    let ctx = PaintingContext {
      canvas: &mut self.canvas,
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
