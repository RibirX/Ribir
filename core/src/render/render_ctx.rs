use crate::render::render_tree::*;
use crate::render::*;
use canvas::{Canvas, FontInfo, Rect, Text};

use std::collections::HashSet;
pub struct RenderCtx<'a> {
  tree: &'a mut RenderTree,
  canvas: &'a mut Canvas,
}

impl<'a> RenderCtx<'a> {
  #[inline]
  pub fn new(tree: &'a mut RenderTree, canvas: &'a mut Canvas) -> RenderCtx<'a> {
    RenderCtx { tree, canvas }
  }

  #[inline]
  pub fn render_object(&self, id: RenderId) -> Option<&(dyn RenderObjectSafety + Send + Sync)> {
    id.get(self.tree)
  }

  /// mark the render object dirty, will auto diffuse to all the node
  /// affected.
  pub fn mark_layout_dirty(&mut self, mut node_id: RenderId) {
    if self.is_layout_dirty(node_id) {
      return;
    }
    loop {
      self.mark_dirty_down(node_id);
      let parent_id = node_id.parent(self.tree);
      if parent_id.is_none() {
        break;
      }
      let constraints = parent_id
        .and_then(|id| id.get(self.tree))
        .map(|node| node.get_constraints())
        .unwrap();
      if !constraints.contains(LayoutConstraints::EFFECTED_BY_CHILDREN) {
        break;
      }
      node_id = parent_id.unwrap();
    }
    node_id.as_dirty_root(self.tree);
  }

  /// perform layout of all node ignore the cache layout info when force is
  /// true, else perform layout just the dirty layout node
  pub fn layout_tree(&mut self, force: bool) {
    if force {
      self.tree.clean_layout_info();
      self.tree.root().map(|node| node.as_dirty_root(self.tree));
    }
    let mut_ptr = self as *mut RenderCtx;
    for root in self.tree.drain_layout_roots() {
      unsafe {
        (*mut_ptr).perform_layout(root);
      }
    }
  }

  /// proxy call the renderObject's perform_layout if needed
  pub fn perform_layout(&mut self, id: RenderId) -> Size {
    let size = self.get_layout_size(id);
    if UNVALID_SIZE != size {
      return size;
    }
    let mut_ptr = self as *mut RenderCtx<'a>;
    let node = id.clone().get_mut(self.tree).unwrap();
    unsafe {
      return node.perform_layout(id, &mut *mut_ptr);
    }
  }

  /// return the layout size. lazy perform layout, if the size has been decided.
  pub fn query_layout_size(&mut self, id: RenderId) -> Size {
    let mut size = self.get_layout_size(id);
    if size == UNVALID_SIZE {
      size = self.perform_layout(id);
    }
    return size;
  }

  // mesure test bound
  // todo support custom font
  pub fn mesure_text(&mut self, text: &str) -> Rect {
    let font = FontInfo::default();
    self.canvas.mesure_text(&Text {
      text,
      font_size: 14.0,
      font,
    })
  }

  pub fn collect_children(&mut self, id: RenderId, ids: &mut Vec<RenderId>) {
    let mut child = id.first_child(self.tree);
    while let Some(child_id) = child {
      ids.push(child_id);
      child = child_id.next_sibling(self.tree);
    }
  }

  pub fn get_box_limit(&self, id: RenderId) -> Option<LimitBox> { id.get_box_limit(&self.tree) }

  pub fn set_box_limit(&mut self, id: RenderId, bound: Option<LimitBox>) {
    id.set_box_limit(&mut self.tree, bound);
  }

  #[inline]
  pub fn update_child_pos(&mut self, child: RenderId, pos: Point) {
    child.update_position(self.tree, pos);
  }

  #[inline]
  pub fn update_size(&mut self, id: RenderId, size: Size) { id.update_size(self.tree, size); }

  #[inline]
  pub fn box_rect(&self, id: RenderId) -> Option<&Rect> { id.box_rect(self.tree) }

  pub(crate) fn get_layout_size(&self, node_id: RenderId) -> Size {
    node_id
      .box_rect(&self.tree)
      .map(|rect| rect.size)
      .unwrap_or(UNVALID_SIZE)
  }

  /// get the layout dirty flag.
  #[inline]
  pub(crate) fn is_layout_dirty(&self, node_id: RenderId) -> bool {
    UNVALID_SIZE == self.get_layout_size(node_id)
  }

  fn mark_dirty_down(&mut self, mut id: RenderId) {
    if self.is_layout_dirty(id) {
      return;
    }
    id.update_size(self.tree, Size::new(-1.0, -1.0));
    let mut ids = vec![];
    self.collect_children(id, &mut ids);
    while let Some(i) = ids.pop() {
      id = i;
      if self.mark_constraints_dirty(id, LayoutConstraints::EFFECTED_BY_PARENT) {
        self.collect_children(id, &mut ids);
      }
    }
  }

  fn mark_constraints_dirty(&mut self, id: RenderId, target: LayoutConstraints) -> bool {
    let constraints = id
      .get(self.tree)
      .map(|node| node.get_constraints())
      .unwrap();
    if constraints.intersects(target) {
      id.update_size(self.tree, Size::new(-1.0, -1.0));
      true
    } else {
      false
    }
  }
}
