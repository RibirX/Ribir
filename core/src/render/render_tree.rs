use crate::{prelude::*, util::TreeFormatter};

use indextree::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct RenderId(NodeId);

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObjectSafety + Send + Sync>>,
  root: Option<RenderId>,
}

impl RenderTree {
  #[inline]
  pub fn root(&self) -> Option<RenderId> { self.root }

  #[inline]
  pub fn set_root(&mut self, root: RenderId) {
    debug_assert!(self.root.is_none());
    self.root = Some(root);
  }

  #[inline]
  pub fn new_node(
    &mut self,
    data: Box<dyn RenderObjectSafety + Send + Sync>,
  ) -> RenderId {
    RenderId(self.arena.new_node(data))
  }

  #[allow(dead_code)]
  pub(crate) fn symbol_shape(&self) -> String {
    if let Some(root) = self.root {
      format!("{:?}", TreeFormatter::new(&self.arena, root.0))
    } else {
      "".to_owned()
    }
  }
}

impl RenderId {
  /// Returns a reference to the node data.
  pub fn get<'a>(
    &self,
    tree: &'a RenderTree,
  ) -> Option<&'a (dyn RenderObjectSafety + Send + Sync)> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut<'a>(
    &mut self,
    tree: &'a mut RenderTree,
  ) -> Option<&'a mut (dyn RenderObjectSafety + Send + Sync + 'static)> {
    tree.arena.get_mut(self.0).map(|node| &mut **node.get_mut())
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  pub fn append(self, new_child: RenderId, tree: &mut RenderTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  pub fn remove(self, tree: &mut RenderTree) { self.0.remove(&mut tree.arena); }

  /// A delegate for [NodeId::parent](indextree::NodeId.parent)
  pub fn parent(&self, tree: &RenderTree) -> Option<RenderId> {
    self.node_id_feature(tree, |node| node.parent())
  }

  /// A delegate for [NodeId::first_child](indextree::NodeId.first_child)
  pub fn first_child(&self, tree: &RenderTree) -> Option<RenderId> {
    self.node_id_feature(tree, |node| node.first_child())
  }

  /// A delegate for [NodeId::last_child](indextree::NodeId.last_child)
  pub fn last_child(&self, tree: &RenderTree) -> Option<RenderId> {
    self.node_id_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub fn previous_sibling(&self, tree: &RenderTree) -> Option<RenderId> {
    self.node_id_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub fn next_sibling(&self, tree: &RenderTree) -> Option<RenderId> {
    self.node_id_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::detach](indextree::NodeId.detach)
  pub fn detach(&self, tree: &mut RenderTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(*self) {
      tree.root = None;
    }
  }

  fn node_id_feature<
    F: Fn(&Node<Box<dyn RenderObjectSafety + Send + Sync>>) -> Option<NodeId>,
  >(
    &self,
    tree: &RenderTree,
    method: F,
  ) -> Option<RenderId> {
    tree
      .arena
      .get(self.0)
      .map(method)
      .flatten()
      .map(|id| RenderId(id))
  }
}
