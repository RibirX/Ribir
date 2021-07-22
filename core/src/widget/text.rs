use crate::prelude::*;
use crate::render::render_ctx::*;
use crate::render::render_tree::*;
use crate::render::*;

/// Just a stupid implement for develope the framework.
#[stateful]
#[derive(Debug, Widget)]
pub struct Text(#[state] pub String);

impl RenderWidget for Text {
  type RO = TextState;
  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone_states() }
}

impl RenderObject for TextState {
  type States = TextState;

  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let rc = ctx.mesure_text(&self.0);
    clamp.clamp(rc.size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn update(&mut self, states: Self::States, _: &mut UpdateCtx) { *self = states; }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let painter = ctx.painter();
    painter.fill_text(&self.0, None);
  }

  #[inline]
  fn get_states(&self) -> &Self::States { self }
}
