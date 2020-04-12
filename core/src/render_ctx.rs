use crate::application::Application;
use crate::render_object::RenderObject;
use crate::render_object_box::LayoutConstraints;

use indextree::*;

use std::collections::HashSet;
pub struct RenderCtx<'a> {
  pub tree: &'a mut Arena<Box<dyn RenderObject + Send + Sync>>,
  dirty_layouts: &'a mut HashSet<NodeId>,
  dirty_layout_roots: &'a mut HashSet<NodeId>,
}

impl<'a> RenderCtx<'a> {
  pub fn new(
    tree: &'a mut Arena<Box<dyn RenderObject + Send + Sync>>,
    dirty_layouts: &'a mut HashSet<NodeId>,
    dirty_layout_roots: &'a mut HashSet<NodeId>,
  ) -> RenderCtx<'a> {
    return RenderCtx {
      tree: tree,
      dirty_layouts: dirty_layouts,
      dirty_layout_roots: dirty_layout_roots,
    };
  }

  pub fn get_render_obj(
    &self,
    id: NodeId,
  ) -> &(dyn RenderObject + Send + Sync) {
    self.tree[id].get().as_ref()
  }

  pub fn collect_children_box(&mut self, id: NodeId, ids: &mut Vec<NodeId>) {
    let mut children_ids = vec![];
    self.collect_children(id, &mut children_ids);
    while children_ids.len() > 0 {
      let id = children_ids.pop().unwrap();
      let render_box = self.get_render_obj(id).to_render_box();
      if render_box.is_none() {
        self.collect_children(id, &mut children_ids);
      } else {
        ids.push(id);
      }
    }
  }

  pub fn get_render_box_id(&self, node_id: NodeId) -> Option<NodeId> {
    let mut id = node_id;
    loop {
      let current_node = &self.tree[id];
      let render_box = current_node.get().to_render_box();
      if render_box.is_some() {
        return Some(id);
      }
      let parent = current_node.parent();
      if parent.is_none() {
        break;
      }
      id = parent.unwrap();
    }
    return None;
  }
  pub fn get_layout_constraints(
    &self,
    node_id: NodeId,
  ) -> Option<LayoutConstraints> {
    return self.tree[node_id]
      .get()
      .to_render_box()
      .map(|node| node.get_constraints());
  }

  pub fn get_parent_box_id(&mut self, node_id: NodeId) -> Option<NodeId> {
    return self
      .get_render_box_id(node_id)
      .and_then(|box_id| self.tree[box_id].parent())
      .and_then(|id| self.get_render_box_id(id));
  }

  pub fn mark_layout_dirty(&mut self, node_id: NodeId) {
    self.dirty_layouts.insert(node_id);
  }

  pub fn is_layout_dirty(&self, node_id: &NodeId) -> bool {
    return self.dirty_layouts.contains(node_id);
  }

  pub fn clear_layout_dirty(&mut self, node_id: &NodeId) {
    self.dirty_layouts.remove(node_id);
  }

  pub fn add_layout_sub_tree(&mut self, node_id: NodeId) {
    self.dirty_layout_roots.insert(node_id);
  }

  pub fn clear_all_dirty_layout(&mut self) {
    self.dirty_layout_roots.clear();
    self.dirty_layouts.clear();
  }

  fn collect_children(&mut self, id: NodeId, ids: &mut Vec<NodeId>) {
    let children = id.reverse_children(self.tree);
    for child in children {
      ids.push(child);
    }
  }
}
