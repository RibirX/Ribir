use indextree::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct Relationship {
  /// A hash map to mapping a render widget in widget tree to its corresponds
  /// render object in render tree.
  widget_to_render: HashMap<NodeId, NodeId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<NodeId, NodeId>,
}

impl Relationship {
  pub fn bind(&mut self, wid: NodeId, rid: NodeId) {
    self.widget_to_render.insert(wid, rid);
    self.render_to_widget.insert(rid, wid);
  }

  pub fn unbind(&mut self, wid: NodeId) {
    let rid = self.widget_to_render.remove(&wid);
    if let Some(rid) = rid {
      let _w = self.render_to_widget.remove(&rid);
      debug_assert!(
        _w.is_some(),
        "widget render and render object must ba a pair"
      )
    }
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    debug_assert!(self.render_to_widget.is_empty());
    self.widget_to_render.is_empty()
  }

  #[inline]
  pub fn widget_to_render(&self, wid: NodeId) -> Option<&NodeId> {
    self.widget_to_render.get(&wid)
  }
}
