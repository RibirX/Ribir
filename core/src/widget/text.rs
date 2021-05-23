use crate::prelude::*;
use crate::render::render_ctx::*;
use crate::render::render_tree::*;
use crate::render::*;

/// Just a stupid implement for develope the framework.
#[derive(Debug, Widget, Stateful)]
pub struct Text(#[state] pub String);

pub struct TextRender(TextState);

impl RenderWidget for Text {
  type RO = TextRender;
  #[inline]
  fn create_render_object(&self) -> Self::RO { TextRender(self.clone_states()) }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[Box<dyn Widget>; 1]>> { None }
}

impl RenderObject for TextRender {
  type States = TextState;

  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let rc = ctx.mesure_text(&self.0.0);
    clamp.clamp(rc.size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn update<'a>(&mut self, states: Self::States, ctx: &mut UpdateCtx) { self.0 = states; }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let painter = ctx.painter();
    painter.fill_text(&self.0.0, None);
  }
}
