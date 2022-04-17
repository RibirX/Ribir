use crate::prelude::*;
use bitflags::bitflags;
use indextree::*;
use std::{collections::HashMap, pin::Pin};

bitflags! {
  pub struct WidgetChangeFlags: u8 {
      const UNSILENT  = 0b00000001;
      const DIFFUSE = 0b00000010;

      const ALL = WidgetChangeFlags::UNSILENT.bits | WidgetChangeFlags::DIFFUSE.bits;
  }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
pub(crate) struct WidgetTree {
  arena: Arena<WidgetNode>,
  root: WidgetId,
  changed_widget: HashMap<WidgetId, WidgetChangeFlags, ahash::RandomState>,
}

impl WidgetTree {
  pub(crate) fn new(w: WidgetNode) -> Pin<Box<Self>> {
    let mut arena = Arena::default();
    let root = WidgetId(arena.new_node(w));
    let tree = Self {
      arena,
      root,
      changed_widget: <_>::default(),
    };
    let mut tree = Box::pin(tree);
    tree.as_mut().state_info_assign(root);
    tree
  }

  #[inline]
  pub(crate) fn root(&self) -> WidgetId { self.root }

  pub(crate) fn new_node(mut self: Pin<&mut Self>, widget: WidgetNode) -> WidgetId {
    let id = WidgetId(self.arena.new_node(widget));
    self.state_info_assign(id);
    id
  }

  #[cfg(test)]
  pub(crate) fn count(&self) -> usize { self.arena.count() }

  // If the widget back of `id` have same `key` with `w` Use `w`, it's will be
  // replaced, otherwise the sub tree of `id` will be detached and insert `w` to
  // replace it.
  pub(crate) fn replace_widget(
    mut self: Pin<&mut Self>,
    w: WidgetNode,
    id: WidgetId,
  ) -> Option<WidgetId> {
    let old = id.assert_get_mut(self.as_mut().get_mut());

    match (old.get_key(), w.get_key()) {
      (Some(k1), Some(k2)) if k1 == k2 => {
        *old = w;
        None
      }
      _ => {
        let parent = id
          .parent(self.as_ref().get_ref())
          .expect("parent should exists!");
        let new_id = parent.append_widget(w, self);
        Some(new_id)
      }
    }
  }

  pub(crate) fn record_change(&mut self, id: WidgetId, flag: WidgetChangeFlags) {
    self
      .changed_widget
      .entry(id)
      .and_modify(|s| {
        *s = *s | flag;
      })
      .or_insert(flag);
  }

  pub(crate) fn pop_changed_widgets(&mut self) -> Option<(WidgetId, WidgetChangeFlags)> {
    self
      .changed_widget
      .keys()
      .next()
      .cloned()
      .and_then(|id| self.changed_widget.remove_entry(&id))
  }

  fn state_info_assign(mut self: Pin<&mut Self>, id: WidgetId) {
    let ptr = std::ptr::NonNull::from(&*self);
    let self_ref = self.as_mut().get_mut();
    if let Some(state_attr) = id.assert_get_mut(self_ref).find_attr_mut::<StateAttr>() {
      state_attr.assign_id(id, ptr);
    }
  }
}

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &WidgetTree) -> Option<&WidgetNode> {
    tree.arena.get(self.0).map(|node| node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut WidgetNode> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &WidgetTree) -> bool { self.0.is_removed(&tree.arena) }

  #[allow(clippy::needless_collect)]
  pub(crate) fn common_ancestor_of(self, other: WidgetId, tree: &WidgetTree) -> Option<WidgetId> {
    if self.is_dropped(tree) || other.is_dropped(tree) {
      return None;
    }

    let p0 = other.ancestors(tree).collect::<Vec<_>>();
    let p1 = self.ancestors(tree).collect::<Vec<_>>();

    p0.iter()
      .rev()
      .zip(p1.iter().rev())
      .filter(|(a, b)| a == b)
      .last()
      .map(|(p, _)| p.clone())
  }

  /// A proxy for [NodeId::parent](indextree::NodeId.parent)
  pub(crate) fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  /// A proxy for [NodeId::first_child](indextree::NodeId.first_child)
  pub(crate) fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  /// A proxy for [NodeId::last_child](indextree::NodeId.last_child)
  pub(crate) fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  /// A proxy for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub(crate) fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  /// A proxy for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub(crate) fn ancestors(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// Detect if this widget is the ancestors of `w`
  pub(crate) fn ancestors_of(self, w: WidgetId, tree: &WidgetTree) -> bool {
    w.ancestors(tree).any(|a| a == self)
  }

  pub(crate) fn children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn reverse_children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.reverse_children(&tree.arena).map(WidgetId)
  }

  /// A proxy for [NodeId::descendants](indextree::NodeId.descendants)

  pub(crate) fn descendants(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  pub(crate) fn remove_subtree(self, tree: &mut WidgetTree) {
    self.0.remove_subtree(&mut tree.arena);
  }

  /// A proxy for [NodeId::detach](indextree::NodeId.detach)
  pub(crate) fn detach(self, tree: &mut WidgetTree) { self.0.detach(&mut tree.arena); }

  pub(crate) fn attach(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn append_widget(self, data: WidgetNode, mut tree: Pin<&mut WidgetTree>) -> WidgetId {
    let id = tree.as_mut().new_node(data);
    self.0.append(id.0, &mut tree.get_mut().arena);
    id
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &WidgetTree) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree));
    self.first_child(tree)
  }

  /// Return the correspond render widget, or down to its single child to find a
  /// nearest render widget from its single descendants.
  pub(crate) fn render_widget(self, tree: &WidgetTree) -> Option<WidgetId> {
    let mut wid = Some(self);
    while let Some(id) = wid {
      wid = match id.assert_get(tree) {
        WidgetNode::Combination(_) => id.single_child(tree),
        _ => break,
      };
    }

    wid
  }

  fn node_feature<F: Fn(&Node<WidgetNode>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub(crate) fn assert_get(self, tree: &WidgetTree) -> &WidgetNode {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut WidgetNode {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }
}

pub(crate) enum WidgetNode {
  Combination(Box<dyn CombinationNode>),
  Render(Box<dyn RenderNode>),
}

impl AsAttrs for WidgetNode {
  fn as_attrs(&self) -> Option<&Attributes> {
    match self {
      WidgetNode::Combination(c) => c.as_attrs(),
      WidgetNode::Render(r) => r.as_attrs(),
    }
  }

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> {
    match self {
      WidgetNode::Combination(c) => c.as_attrs_mut(),
      WidgetNode::Render(r) => r.as_attrs_mut(),
    }
  }
}

impl WidgetNode {
  pub fn find_attr<A: 'static>(&self) -> Option<&A> {
    match self {
      WidgetNode::Combination(c) => c.as_attrs(),
      WidgetNode::Render(r) => r.as_attrs(),
    }
    .and_then(Attributes::find)
  }

  pub fn find_attr_mut<A: 'static>(&mut self) -> Option<&mut A> {
    match self {
      WidgetNode::Combination(c) => c.as_attrs_mut(),
      WidgetNode::Render(r) => r.as_attrs_mut(),
    }
    .and_then(Attributes::find_mut)
  }
}

impl AsAttrs for BoxedWidgetInner {
  fn as_attrs(&self) -> Option<&Attributes> {
    match self {
      BoxedWidgetInner::Combination(c) => c.as_attrs(),
      BoxedWidgetInner::Render(r) => r.as_attrs(),
      BoxedWidgetInner::SingleChild(s) => s.as_attrs(),
      BoxedWidgetInner::MultiChild(m) => m.as_attrs(),
    }
  }

  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> {
    match self {
      BoxedWidgetInner::Combination(c) => c.as_attrs_mut(),
      BoxedWidgetInner::Render(r) => r.as_attrs_mut(),
      BoxedWidgetInner::SingleChild(s) => s.as_attrs_mut(),
      BoxedWidgetInner::MultiChild(m) => m.as_attrs_mut(),
    }
  }
}
