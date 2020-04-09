use crate::render_object::RenderObject;
use crate::render_object_box::RenderObjectBox;
use indextree::*;
pub struct RenderCtx<'a> {
  pub tree: &'a mut Arena<Box<dyn RenderObject>>,
}

impl<'a> RenderCtx<'a> {
  pub fn new(tree: &'a mut Arena<Box<dyn RenderObject>>) -> RenderCtx<'a> {
    return RenderCtx { tree: tree };
  }

  pub fn collect_children(&mut self, id: NodeId, ids: &mut Vec<NodeId>) {
    let children = id.reverse_children(self.tree);
    for child in children {
      ids.push(child);
    }
  }

  pub fn get_render_box(
    &mut self,
    id: NodeId,
  ) -> Option<&mut dyn RenderObjectBox> {
    let node = self.tree.get_mut(id).unwrap();
    return node.get_mut().to_render_box();
  }

  pub fn collect_children_box(&mut self, id: NodeId, ids: &mut Vec<NodeId>) {
    let mut children_ids = vec![];
    self.collect_children(id, &mut children_ids);
    while children_ids.len() > 0 {
      let id = children_ids.pop().unwrap();
      let render_box = self.get_render_box(id);
      if render_box.is_none() {
        self.collect_children(id, &mut children_ids);
      } else {
        ids.push(id);
      }
    }
  }

  pub fn get_render_box_id(&mut self, node_id: NodeId) -> Option<NodeId> {
    let mut id = node_id;
    loop {
      let current_node = self.tree.get_mut(id).unwrap();
      let render_box = current_node.get_mut().to_render_box();
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
  pub fn get_parent_box_id(&mut self, node_id: NodeId) -> Option<NodeId> {
    return self.tree.get(node_id).and_then(|node| node.parent());
  }
}
