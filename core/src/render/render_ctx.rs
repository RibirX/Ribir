use crate::render::render_tree::*;
use crate::render::*;

use std::collections::HashSet;
pub struct RenderCtx<'a> {
  tree: &'a mut RenderTree,
  dirty_layouts: &'a mut HashSet<RenderId>,
  dirty_layout_roots: &'a mut HashSet<RenderId>,
}

impl<'a> RenderCtx<'a> {
  #[inline]
  pub fn new(
    tree: &'a mut RenderTree,
    dirty_layouts: &'a mut HashSet<RenderId>,
    dirty_layout_roots: &'a mut HashSet<RenderId>,
  ) -> RenderCtx<'a> {
    RenderCtx {
      tree,
      dirty_layouts,
      dirty_layout_roots,
    }
  }

  #[inline]
  pub fn render_object(&self, id: RenderId) -> Option<&(dyn RenderObjectSafety + Send + Sync)> {
    id.get(self.tree)
  }

  /// return the render tree
  #[inline]
  pub fn render_tree(&self) -> &RenderTree { &self.tree }

  /// mark the render object dirty, will auto diffuse to all the node
  /// affected.
  pub fn mark_layout_dirty(&mut self, mut node_id: RenderId) {
    if self.is_layout_dirty(node_id) {
      return;
    }
    self.dirty_layouts.insert(node_id);
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
    self.dirty_layout_roots.insert(node_id);
  }

  /// perform layout of all node ignore the cache layout info when force is
  /// true, else perform layout just the dirty layout node
  pub fn layout_tree(&mut self, force: bool) {
    if force {
      todo!("force all tree relayout");
    } else {
      let mut_ptr = self as *mut RenderCtx;
      for root in self.dirty_layout_roots.drain() {
        unsafe {
          (*mut_ptr).perform_layout(root);
        }
      }
    }
  }

  /// proxy call the renderObject's perform_layout if needed
  pub fn perform_layout(&mut self, id: RenderId) {
    if !self.is_layout_dirty(id) {
      return;
    }
    let mut_ptr = self as *mut RenderCtx<'a>;
    let node = id.clone().get_mut(self.tree).unwrap();
    unsafe {
      node.perform_layout(id, &mut *mut_ptr);
    }

    self.remove_layout_dirty(id);
  }

  /// get the layout dirty flag.
  #[inline]
  pub fn is_layout_dirty(&self, node_id: RenderId) -> bool { self.dirty_layouts.contains(&node_id) }

  /// remove the layout dirty flag.
  pub fn remove_layout_dirty(&mut self, node_id: RenderId) { self.dirty_layouts.remove(&node_id); }

  pub fn collect_children(&mut self, id: RenderId, ids: &mut Vec<RenderId>) {
    let mut child = id.first_child(self.tree);
    while let Some(child_id) = child {
      ids.push(child_id);
      child = id.next_sibling(self.tree);
    }
  }

  pub fn set_box_limit(&mut self, id: RenderId, bound: Option<BoxLimit>) {
    id.clone().get_mut(self.tree).unwrap().set_box_limit(bound);
  }

  fn mark_dirty_down(&mut self, mut id: RenderId) {
    if self.is_layout_dirty(id) {
      return;
    }
    self.dirty_layouts.insert(id);
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
      self.dirty_layouts.insert(id);
      true
    } else {
      false
    }
  }
}
