use super::render_tree::*;
use crate::prelude::Point;
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
    fn assert_place(id: RenderId, tree: &RenderTree) -> Point {
      id.layout_box_rect(tree)
        .expect("Every widget should at its place before draw.")
        .min()
    }
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

          let offset = assert_place(id, &self.tree);
          self.layer_2d.translate(offset.x, offset.y);

          r_obj.paint(&mut self);
        }
        RenderEdge::End(id) => {
          let offset = assert_place(id, &self.tree);
          self.layer_2d.translate(-offset.x, -offset.y);
        }
      });

    self.layer_2d
  }
}
