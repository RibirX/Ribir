use crate::prelude::*;
use crate::render_object::RenderCtx;

/// Just a stupid implement for develope the framework.

pub struct Text(pub &'static str);

impl RenderObject for Text {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String { format!("RO::Text({})", self.0) }
  fn paint(&self) {}
  fn perform_layout(&mut self, _ctx: RenderCtx) {}
}

impl From<Text> for Widget {
  fn from(t: Text) -> Self { Widget::Render(Box::new(t)) }
}

impl<'a> RenderWidget<'a> for Text {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String { format!("text({})", self.0) }
  fn create_render_object(&self) -> Box<dyn RenderObject> {
    Box::new(Text(self.0))
  }
}
