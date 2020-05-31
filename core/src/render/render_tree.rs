use crate::{prelude::*, util::TreeFormatter, widget::widget_tree::*};
use indextree::*;
use std::collections::HashMap;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct RenderId(NodeId);
pub enum RenderEdge {
  Start(RenderId),
  End(RenderId),
}

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObjectSafety + Send + Sync>>,
  root: Option<RenderId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<RenderId, WidgetId>,
}

impl RenderTree {
  #[inline]
  pub fn root(&self) -> Option<RenderId> { self.root }

  pub fn set_root(
    &mut self,
    owner: WidgetId,
    data: Box<dyn RenderObjectSafety + Send + Sync>,
  ) -> RenderId {
    debug_assert!(self.root.is_none());
    let root = self.new_node(data);
    self.root = Some(root);
    self.render_to_widget.insert(root, owner);
    root
  }

  #[inline]
  pub fn new_node(&mut self, data: Box<dyn RenderObjectSafety + Send + Sync>) -> RenderId {
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

  #[cfg(test)]
  pub(crate) fn render_to_widget(&self) -> &HashMap<RenderId, WidgetId> { &self.render_to_widget }
}

impl RenderId {
  /// Returns a reference to the node data.
  pub(crate) fn get<'a>(
    self,
    tree: &'a RenderTree,
  ) -> Option<&'a (dyn RenderObjectSafety + Send + Sync)> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut<'a>(
    mut self,
    tree: &'a mut RenderTree,
  ) -> Option<&'a mut (dyn RenderObjectSafety + Send + Sync + 'static)> {
    tree.arena.get_mut(self.0).map(|node| &mut **node.get_mut())
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  #[inline]
  pub(crate) fn append(self, new_child: RenderId, tree: &mut RenderTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::preend](indextree::NodeId.preend)
  #[inline]
  pub(crate) fn prepend(self, new_child: RenderId, tree: &mut RenderTree) {
    self.0.prepend(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  #[inline]
  pub(crate) fn remove(self, tree: &mut RenderTree) { self.0.remove(&mut tree.arena); }

  /// Returns an iterator of references to this node’s children.
  pub(crate) fn children<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.children(&tree.arena).map(|id| RenderId(id))
  }

  /// Returns an iterator of references to this node and its descendants, in
  /// tree order.
  pub(crate) fn traverse<'a>(&self, tree: &'a RenderTree) -> impl Iterator<Item = RenderEdge> + 'a {
    self.0.traverse(&tree.arena).map(|edge| match edge {
      NodeEdge::Start(id) => RenderEdge::Start(RenderId(id)),
      NodeEdge::End(id) => RenderEdge::End(RenderId(id)),
    })
  }

  /// A delegate for [NodeId::parent](indextree::NodeId.parent)
  pub(crate) fn parent(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.parent())
  }

  /// A delegate for [NodeId::first_child](indextree::NodeId.first_child)
  pub(crate) fn first_child(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.first_child())
  }

  /// A delegate for [NodeId::last_child](indextree::NodeId.last_child)
  pub(crate) fn last_child(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub(crate) fn previous_sibling(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub(crate) fn next_sibling(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub(crate) fn ancestors<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.ancestors(&tree.arena).map(|id| RenderId(id))
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  pub(crate) fn descendants<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.descendants(&tree.arena).map(|id| RenderId(id))
  }

  /// Preappend a RenderObject as child, and create this RenderObject's Widget
  /// is `owner`
  pub(crate) fn prepend_object(
    self,
    owner: WidgetId,
    object: Box<dyn RenderObjectSafety + Send + Sync>,
    tree: &mut RenderTree,
  ) -> RenderId {
    let child = tree.new_node(object);
    self.prepend(child, tree);
    tree.render_to_widget.insert(child, owner);
    child
  }

  /// Drop the subtree
  pub(crate) fn drop(self, tree: &mut RenderTree) {
    let RenderTree {
      render_to_widget,
      arena,
      ..
    } = tree;
    self.0.descendants(arena).for_each(|id| {
      render_to_widget.remove(&RenderId(id));
    });

    // Todo: should remove in a more directly way and not care about
    // relationship
    // Fixme: memory leak here, node just detach and not remove. Wait a pr to
    // provide a method to drop a subtree in indextree.
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  /// return the relative render widget.
  pub(crate) fn relative_to_widget(self, tree: &mut RenderTree) -> Option<WidgetId> {
    tree.render_to_widget.get(&self).map(|id| *id)
  }

  fn node_feature<F: Fn(&Node<Box<dyn RenderObjectSafety + Send + Sync>>) -> Option<NodeId>>(
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
