use super::render_tree::*;
use canvas::Rendering2DLayer;

pub type Painter2D<'a> = Rendering2DLayer<'a>;

pub struct PaintingContext<'a> {
  layer_2d: Rendering2DLayer<'a>,
  current_node: RenderId,
  tree: &'a RenderTree,
}

impl<'a> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(tree: &'a RenderTree) -> Option<Self> {
    tree.root().map(|root| Self {
      layer_2d: Rendering2DLayer::new(),
      current_node: root,
      tree,
    })
  }

  /// Return the 2d painter to draw 2d things.
  pub fn painter(&mut self) -> &mut Painter2D<'a> { &mut self.layer_2d }

  pub(crate) fn draw(mut self) -> Rendering2DLayer<'a> {
    let mut stack = vec![(self.current_node, 1, None)];

    let mut child_index = 0;
    self
      .current_node
      .traverse(&self.tree)
      .for_each(|edge| match edge {
        RenderEdge::Start(id) => {
          self.current_node = id;
          let r_obj = self
            .current_node
            .get(&self.tree)
            .expect("Render object should exists when traverse the tree.");

          let offset = id
            .parent(&self.tree)
            .map(|p| p.get(&self.tree))
            .flatten()
            .map(|obj| obj.child_offset(child_index))
            .flatten();

          if let Some(ref offset) = offset {
            self.layer_2d.translate(offset.x, offset.y);
          }
          r_obj.paint(&mut self);

          stack.push((id, child_index, offset));
          child_index = 0;
        }
        RenderEdge::End(id) => {
          if let Some((start_id, index, offset)) = stack.pop() {
            debug_assert_eq!(id, start_id);
            child_index = index + 1;

            if let Some(ref offset) = offset {
              self.layer_2d.translate(-offset.x, -offset.y);
            }
          }
        }
      });

    self.layer_2d
  }
}
