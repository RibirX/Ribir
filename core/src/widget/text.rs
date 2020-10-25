use crate::prelude::*;
use crate::render::render_ctx::*;
use crate::render::render_tree::*;
use crate::render::*;

/// Just a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Text(pub String);

impl_widget_for_render_widget!(Text);

#[derive(Debug)]
pub struct TextRender {
  text: String,
}

impl RenderWidget for Text {
  type RO = TextRender;
  fn create_render_object(&self) -> Self::RO {
    TextRender {
      text: self.0.clone(),
    }
  }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> { None }
}

impl RenderObject for TextRender {
  type Owner = Text;
  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let rc = ctx.mesure_text(&self.text);
    clamp.clamp(rc.size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn update<'a>(&mut self, owner_widget: &Text, ctx: &mut UpdateCtx) {
    if self.text != owner_widget.0 {
      self.text = owner_widget.0.clone();
      ctx.mark_needs_layout();
    }
  }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let painter = ctx.painter();
    painter.fill_text(&self.text, None);
  }
}
