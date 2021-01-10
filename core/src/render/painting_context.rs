use super::{render_tree::*, RenderObjectSafety};
use crate::prelude::*;
use canvas::Rendering2DLayer;

pub type Painter2D<'a> = Rendering2DLayer<'a>;

pub struct PaintingContext<'a> {
  layer_2d: Rendering2DLayer<'a>,
  current_node: RenderId,
  tree: &'a RenderTree,
}

impl<'a> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(tree: &'a RenderTree, transform: Transform) -> Option<Self> {
    tree.root().map(|root| {
      let mut layer_2d = Rendering2DLayer::new();
      layer_2d.set_transform(transform);
      Self {
        layer_2d,
        current_node: root,
        tree,
      }
    })
  }

  /// Return the 2d painter to draw 2d things.
  pub fn painter(&mut self) -> &mut Painter2D<'a> { &mut self.layer_2d }

  /// Return the size of the render object occupied after perform layout.
  pub fn self_size(&self) -> Option<Size> {
    self
      .current_node
      .layout_box_rect(self.tree)
      .map(|rect| rect.size)
  }

  /// Return an iterator of children's box rect relative to this widget.
  pub fn children_rect(&self) -> impl Iterator<Item = Rect> + '_ {
    self.current_node.children(self.tree).map(move |rid| {
      rid
        .layout_box_rect(self.tree)
        .expect("children must already layout when paint.")
    })
  }

  pub(crate) fn draw(mut self) -> Rendering2DLayer<'a> {
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

          self.layer_2d.save();
          let offset = id
            .layout_box_rect(&self.tree)
            .expect("Every widget should at its place before draw.")
            .min();

          let mut matrix = self
            .layer_2d
            .get_transform()
            .pre_translate(offset.to_vector());

          if let Some(t) = id.get(&self.tree).and_then(RenderObjectSafety::transform) {
            matrix = matrix.then(&t);
          }
          self.layer_2d.set_transform(matrix);

          r_obj.paint(&mut self);
        }
        RenderEdge::End(_) => {
          self.layer_2d.restore();
        }
      });

    self.layer_2d
  }
}
