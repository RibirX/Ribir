use crate::prelude::*;
use crate::render::render_ctx::*;
use crate::render::render_layout::*;
use crate::render::render_tree::*;
use crate::render::*;
use indextree::*;

/// Just a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Text(pub &'static str);

impl Widget for Text {
  render_widget_base_impl!();
}
#[derive(Debug)]
pub struct TextRender(&'static str);

impl RenderWidget for Text {
  type RO = TextRender;
  fn create_render_object(&self) -> Self::RO { TextRender(self.0) }
}

impl RenderObject<Text> for TextRender {
  fn update<'a>(&mut self, owner_widget: &Text) { self.0 = owner_widget.0; }
  fn perform_layout(&self, id: RenderId, ctx: &mut RenderCtx) {}
  fn bound(&self) -> Option<Size> { None }
  fn get_constraints(&self) -> LayoutConstraints {
    LayoutConstraints::DECIDED_BY_SELF
  }
}
// impl RenderObject for Text {
//   fn paint(&self) {}
//   fn to_render_box(&self) -> Option<&dyn RenderObjectBox> { Some(self) }
//   fn to_render_box_mut(&mut self) -> Option<&mut dyn RenderObjectBox> {
//     Some(self)
//   }
// }

// impl RenderWidget for Text {
//   fn create_render_object(&self) -> Box<dyn RenderObject + Send + Sync> {
//     Box::new(Text(self.0))
//   }
// }

// impl RenderObjectBox for Text {
//   fn bound(&self) -> Option<Size> {
//     return Some(Size {
//       width: self.0.len() as i32,
//       height: 1,
//     });
//   }
//   fn get_constraints(&self) -> LayoutConstraints {
//     LayoutConstraints::DECIDED_BY_SELF
//   }

//   fn layout_sink(&mut self, _self_id: NodeId, _ctx: &mut RenderCtx) {}
//   fn layout_bubble(&mut self, _self_id: NodeId, _ctx: &mut RenderCtx) {}
//   fn mark_dirty(&mut self) {}
//   fn is_dirty(&self) -> bool { return false; }
// }
