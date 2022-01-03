use super::{layout_store::LayoutStore, render_tree::*};
use crate::prelude::*;

pub struct PaintingContext<'a> {
  painter: &'a mut Painter,
  current_node: RenderId,
  layout_store: &'a LayoutStore,
  tree: &'a RenderTree,
}

impl<'a> PaintingContext<'a> {
  #[inline]
  pub(crate) fn new(
    tree: &'a RenderTree,
    painter: &'a mut Painter,
    layout_store: &'a LayoutStore,
  ) -> Self {
    Self {
      painter,
      layout_store,
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
      .layout_store
      .layout_box_rect(self.current_node)
      .map(|rect| rect.size)
  }

  /// Return an iterator of children's box rect relative to this widget.
  pub fn children_rect(&self) -> impl Iterator<Item = Rect> + '_ {
    self.current_node.children(self.tree).map(move |rid| {
      self
        .layout_store
        .layout_box_rect(rid)
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
          let offset = self
            .layout_store
            .layout_box_rect(id)
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

    self.painter.finish()
  }
}
