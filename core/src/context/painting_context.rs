use crate::prelude::{render_tree::RenderEdge, *};

use super::{Context, WidgetCtx};

pub struct PaintingCtx<'a> {
  id: WidgetId,
  ctx: &'a mut Context,
}

impl<'a> PaintingCtx<'a> {
  #[inline]
  pub(crate) fn new(id: WidgetId, ctx: &'a mut Context) -> Self { Self { id, ctx } }

  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { &mut self.ctx.painter }

  /// Return the size of the render object occupied after perform layout.
  pub fn self_size(&self) -> Size {
    let rid = self.id.relative_to_render(&self.ctx.widget_tree).unwrap();
    self
      .ctx
      .layout_store
      .layout_box_rect(rid)
      .map(|rect| rect.size)
      .expect("children must already layout when paint.")
  }

  /// Return an iterator of children's box rect relative to this widget.
  pub fn child_rect(&self, wid: WidgetId) -> Rect {
    let rid = wid.relative_to_render(&self.ctx.widget_tree).unwrap();
    self
      .ctx
      .layout_store
      .layout_box_rect(rid)
      .expect("children must already layout when paint.")
  }
}

pub(crate) fn draw_tree(ctx: &mut Context) -> Vec<PaintCommand> {
  let root = ctx.render_tree.root().expect("try to paint a empty tree");
  let (ctx, traverse) = ctx.split_traverse(root);
  let (ctx, r_tree) = ctx.split_r_tree();

  traverse.for_each(|edge| match edge {
    RenderEdge::Start(id) => {
      let r_obj = id
        .get(r_tree)
        .expect("Render object should exists when traverse the tree.");

      ctx.painter.save();
      let offset = ctx
        .layout_store
        .layout_box_rect(id)
        .expect("Every widget should at its place before draw.")
        .min();

      let mut matrix = ctx
        .painter
        .get_transform()
        .pre_translate(offset.to_vector());

      if let Some(t) = id.get(r_tree).and_then(RenderObject::transform) {
        matrix = matrix.then(&t);
      }
      ctx.painter.set_transform(matrix);

      let id = id.relative_to_widget(r_tree).unwrap();
      let mut ctx = PaintingCtx { id, ctx };
      r_obj.paint(&mut ctx);
    }
    RenderEdge::End(_) => {
      ctx.painter.restore();
    }
  });

  ctx.painter.finish()
}

impl<'a> WidgetCtx<'a> for PaintingCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn context(&self) -> &Context { self.ctx }
}
