use super::{
  painter::{PaintCommand, Painter},
  render_tree::*,
};
use crate::prelude::*;

pub struct PaintingContext<'a> {
  painter: Painter,
  current_node: RenderId,
  tree: &'a RenderTree,
}

impl<'a> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(tree: &'a RenderTree, transform: Transform) -> Self {
    let mut layer_2d = Painter::new();
    layer_2d.set_transform(transform);

    Self {
      painter: layer_2d,
      current_node: tree
        .root()
        .expect("Try to paint a uninit render tree, which root is none"),
      tree,
    }
  }

  /// Return the 2d painter to draw 2d things.
  pub fn painter(&mut self) -> &mut Painter { &mut self.painter }

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

  pub(crate) fn draw(mut self) -> Vec<PaintCommand> {
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

          self.painter.save();
          let offset = id
            .layout_box_rect(&self.tree)
            .expect("Every widget should at its place before draw.")
            .min();

          let mut matrix = self
            .painter
            .get_transform()
            .pre_translate(offset.to_vector());

          if let Some(t) = id.get(&self.tree).and_then(RenderObject::transform) {
            matrix = matrix.then(&t);
          }
          self.painter.set_transform(matrix);

          r_obj.paint(&mut self);
        }
        RenderEdge::End(_) => {
          self.painter.restore();
        }
      });

    self.painter.commands
  }
}
