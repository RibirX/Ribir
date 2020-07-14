use crate::{prelude::*, util::TreeFormatter, widget::widget_tree::*};
use indextree::*;
use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct RenderId(NodeId);
pub enum RenderEdge {
  Start(RenderId),
  End(RenderId),
}

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

/// render object's layout box, the information about layout, including box
/// size, box position, and the clamp of render object layout.
#[derive(Debug, Default)]
pub struct BoxLayout {
  /// Box bound is the bound of the layout can be place. it will be set after
  /// render object computing its layout. It's passed by render object's parent.
  pub clamp: BoxClamp,
  /// The position and size render object to place, relative to its parent
  /// coordinate. Some value after the relative render object has been layout,
  /// otherwise is none value.
  pub rect: Option<Rect>,
}

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObjectSafety + Send + Sync>>,
  root: Option<RenderId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<RenderId, WidgetId>,
  /// Store the render object's place relative to parent coordinate and the
  /// clamp passed from parent.
  layout_info: HashMap<RenderId, BoxLayout>,
  /// root of sub tree which needed to perform layout, store as min-head by the
  /// node's depth.
  needs_layout: BinaryHeap<Reverse<(usize, RenderId)>>,
}

impl BoxClamp {
  #[inline]
  pub fn clamp(self, size: Size) -> Size { size.clamp(self.min, self.max) }
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self {
      min: Size::new(0., 0.),
      max: Size::new(f32::INFINITY, f32::INFINITY),
    }
  }
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
    self.push_relayout_sub_root(root);
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

  #[cfg(test)]
  pub fn layout_info(&self) -> &HashMap<RenderId, BoxLayout> { &self.layout_info }

  fn push_relayout_sub_root(&mut self, rid: RenderId) {
    self
      .needs_layout
      .push(std::cmp::Reverse((rid.ancestors(self).count(), rid)));
  }
}

impl RenderId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &RenderTree) -> Option<&(dyn RenderObjectSafety + Send + Sync)> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(
    self,
    tree: &mut RenderTree,
  ) -> &mut (dyn RenderObjectSafety + Send + Sync + 'static) {
    &mut **tree
      .arena
      .get_mut(self.0)
      .expect("Access a removed render object")
      .get_mut()
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  #[allow(dead_code)]
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
  #[allow(dead_code)]
  #[inline]
  pub(crate) fn remove(self, tree: &mut RenderTree) { self.0.remove(&mut tree.arena); }

  /// Returns an iterator of references to this node’s children.
  #[inline]
  pub(crate) fn children<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.children(&tree.arena).map(RenderId)
  }

  /// Returns an iterator of references to this node’s children.
  pub(crate) fn reverse_children<'a>(
    self,
    tree: &'a RenderTree,
  ) -> impl Iterator<Item = RenderId> + 'a {
    self.0.reverse_children(&tree.arena).map(RenderId)
  }

  /// Returns an iterator of references to this node and its descendants, in
  /// tree order.
  pub(crate) fn traverse<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderEdge> + 'a {
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
    self.0.ancestors(&tree.arena).map(RenderId)
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  pub(crate) fn descendants<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.descendants(&tree.arena).map(RenderId)
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
    tree.layout_info.remove(&self);
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  pub(crate) fn relative_to_widget(self, tree: &RenderTree) -> Option<WidgetId> {
    tree.render_to_widget.get(&self).copied()
  }

  pub(crate) fn layout_clamp(self, tree: &RenderTree) -> Option<BoxClamp> {
    tree.layout_info.get(&self).map(|info| info.clamp)
  }

  pub(crate) fn layout_box_rect(self, tree: &RenderTree) -> Option<Rect> {
    tree.layout_info.get(&self).and_then(|info| info.rect)
  }

  pub(crate) fn layout_clamp_mut(self, tree: &mut RenderTree) -> &mut BoxClamp {
    &mut self.layout_info_mut(&mut tree.layout_info).clamp
  }

  pub(crate) fn layout_box_rect_mut(self, tree: &mut RenderTree) -> &mut Rect {
    self
      .layout_info_mut(&mut tree.layout_info)
      .rect
      .get_or_insert_with(Rect::zero)
  }

  pub(crate) fn mark_needs_layout(self, tree: &mut RenderTree) {
    if self.layout_box_rect(tree).is_none() {
      let mut relayout_root = self;
      let RenderTree {
        arena, layout_info, ..
      } = tree;
      // All ancestors of this render object should relayout until the one which only
      // sized by parent.
      self.0.ancestors(arena).all(|id| {
        let sized_by_parent = arena
          .get(id)
          .map_or(false, |node| node.get().only_sized_by_parent());
        if !sized_by_parent {
          let rid = RenderId(id);
          self.layout_info_mut(layout_info).rect = None;
          relayout_root = rid;
        }

        !sized_by_parent
      });
      tree.push_relayout_sub_root(relayout_root);
    }
  }

  fn layout_info_mut(self, layout_info: &mut HashMap<RenderId, BoxLayout>) -> &mut BoxLayout {
    layout_info.entry(self).or_insert_with(BoxLayout::default)
  }

  fn node_feature<F: Fn(&Node<Box<dyn RenderObjectSafety + Send + Sync>>) -> Option<NodeId>>(
    self,
    tree: &RenderTree,
    method: F,
  ) -> Option<RenderId> {
    tree.arena.get(self.0).map(method).flatten().map(RenderId)
  }
}

impl !Unpin for RenderTree {}
