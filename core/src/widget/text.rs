use crate::prelude::*;
use crate::render_ctx::RenderCtx;
use crate::render_object_box::{LayoutConstraints, RenderObjectBox, Size};
use indextree::*;

/// Just a stupid implement for develope the framework.
#[derive(Debug)]
pub struct Text(pub &'static str);

impl RenderObject for Text {
  fn paint(&self) {}
  fn to_render_box(&mut self) -> Option<&mut dyn RenderObjectBox> { Some(self) }
}

impl<'a> WidgetStates<'a> for Text {}
impl<'a> RenderWidget<'a> for Text {
  fn create_render_object(&self) -> Box<dyn RenderObject> {
    Box::new(Text(self.0))
  }
}

impl RenderObjectBox for Text {
  fn bound(&self) -> Option<Size> {
    return Some(Size {
      width: self.0.len() as i32,
      height: 1,
    });
  }
  fn get_constraints(&self) -> LayoutConstraints {
    LayoutConstraints::DecidedBySelf
  }

  fn layout_sink(&mut self, _ctx: &mut RenderCtx, _self_id: NodeId) {}
  fn layout_bubble(&mut self, _ctx: &mut RenderCtx, _self_id: NodeId) {}
  fn mark_dirty(&mut self) {}
  fn is_dirty(&self) -> bool { return false; }
}
