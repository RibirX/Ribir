use crate::{prelude::*, util::TreeFormatter};

use indextree::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);

#[derive(Default)]
pub struct WidgetTree<'a> {
  arena: Arena<Widget<'a>>,
  root: Option<WidgetId>,
}

impl<'a> WidgetTree<'a> {
  #[inline]
  pub fn root(&self) -> Option<WidgetId> { self.root }

  #[inline]
  pub fn set_root(&mut self, root: WidgetId) {
    debug_assert!(self.root.is_none());
    self.root = Some(root);
  }

  #[inline]
  pub fn new_node(&mut self, data: Widget<'a>) -> WidgetId {
    WidgetId(self.arena.new_node(data))
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

impl WidgetId {
  /// Returns a reference to the node data.
  pub fn get<'a>(self, tree: &'a WidgetTree) -> Option<&'a Widget<'a>> {
    tree.arena.get(self.0).map(|node| node.get())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut<'a, 'b>(
    self,
    tree: &'b mut WidgetTree<'a>,
  ) -> Option<&'b mut Widget<'a>> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  pub fn append(self, new_child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  pub fn remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena); }

  /// A delegate for [NodeId::parent](indextree::NodeId.parent)
  pub fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.parent())
  }

  /// A delegate for [NodeId::first_child](indextree::NodeId.first_child)
  pub fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.first_child())
  }

  /// A delegate for [NodeId::last_child](indextree::NodeId.last_child)
  pub fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub fn previous_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub fn ancestors<'a>(
    self,
    tree: &'a WidgetTree,
  ) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.ancestors(&tree.arena).map(|id| WidgetId(id))
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  pub fn descendants<'a>(
    self,
    tree: &'a WidgetTree,
  ) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.descendants(&tree.arena).map(|id| WidgetId(id))
  }

  /// A delegate for [NodeId::detach](indextree::NodeId.detach)
  pub fn detach(&self, tree: &mut WidgetTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(*self) {
      tree.root = None;
    }
  }

  /// Caller assert this node only have one child, other panic!
  pub(crate) fn single_child(self, tree: &WidgetTree) -> WidgetId {
    debug_assert!(self.first_child(tree).is_some());
    debug_assert_eq!(self.first_child(tree), self.last_child(tree));
    self
      .first_child(tree)
      .expect("Caller assert `wid` has single child")
  }

  /// find the nearest render widget in subtree, include self.
  pub(crate) fn down_nearest_render_widget(
    self,
    tree: &WidgetTree,
  ) -> WidgetId {
    let mut wid = self;
    while let Some(Widget::Combination(_)) = wid.get(tree) {
      wid = wid.single_child(tree);
    }
    debug_assert!(!matches!(&wid.get(&tree).unwrap(), Widget::Combination(_)));
    wid
  }

  /// find the nearest render widget in ancestors, include self.
  pub(crate) fn upper_nearest_render_widget(
    self,
    tree: &WidgetTree,
  ) -> WidgetId {
    let wid = self
      .ancestors(tree)
      .find(|id| !matches!(id.get(tree), Some(Widget::Combination(_))))
      .expect(
        "should only call this method if `wid`  have render widget ancestor!",
      );

    debug_assert!(matches!(wid.get(tree).unwrap(), Widget::Render(_)));

    wid
  }

  fn node_id_feature<F: Fn(&Node<Widget>) -> Option<NodeId>>(
    &self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree
      .arena
      .get(self.0)
      .map(method)
      .flatten()
      .map(|id| WidgetId(id))
  }
}
