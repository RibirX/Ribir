use crate::prelude::*;
use crate::render_object::RenderCtx;

/// Just a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Text(pub &'static str);

impl RenderObject for Text {
  fn paint(&self) {}
  fn perform_layout(&mut self, _ctx: RenderCtx) {}
}

impl From<Text> for Widget {
  fn from(t: Text) -> Self { Widget::Render(Box::new(t)) }
}

impl<'a> RenderWidget<'a> for Text {
  fn create_render_object(&self) -> Box<dyn RenderObject> {
    Box::new(Text(self.0))
  }
}
