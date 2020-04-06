use crate::render_object::RenderObject;
use indextree::*;
pub struct RenderCtx<'a> {
  pub tree: &'a mut Tree<Box<dyn RenderObject>>,
}

impl<'a> RenderCtx<'a> {
  pub fn new(tree: &'a mut Tree<Box<dyn RenderObject>>) -> RenderCtx<'a> {
    return RenderCtx { tree: tree };
  }

  pub fn step_into_child_box_reverse(
    &mut self,
    id: NodeId,
    ids: &mut Vec<NodeId>,
  ) {
    let children = self.tree.get(id).unwrap().children();
    let mut children_ids = vec![];
    for child in children {
      children_ids.push(child.node_id());
    }
    while children_ids.len() > 0 {
      let id = children_ids.pop().unwrap();
      let mut node = self.tree.get_mut(id).unwrap();
      let render_box = node.data().to_render_box();
      if render_box.is_none() {
        let children = self.tree.get(id).unwrap().children();
        for child in children {
          children_ids.push(child.node_id());
        }
      } else {
        ids.push(id);
      }
    }
  }

  pub fn get_render_box_id(&mut self, node_id: NodeId) -> Option<NodeId> {
    let mut id = node_id;
    loop {
      let mut current_node = self.tree.get_mut(id).unwrap();
      let render_box = current_node.data().to_render_box();
      if render_box.is_some() {
        return Some(id);
      }
      let parent = current_node.parent();
      if parent.is_none() {
        break;
      }
      id = parent.unwrap().node_id();
    }
    return None;
  }
  pub fn get_parent_box_id(&mut self, node_id: NodeId) -> Option<NodeId> {
    return self
      .tree
      .get(node_id)
      .and_then(|node| node.parent().and_then(|node| Some(node.node_id())));
  }
}
