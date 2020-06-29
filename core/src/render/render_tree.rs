use crate::{prelude::*, util::TreeFormatter, widget::widget_tree::*};
use indextree::*;
use std::collections::hash_set::Drain;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct RenderId(NodeId);
pub enum RenderEdge {
  Start(RenderId),
  End(RenderId),
}

/// boundary limit of the render object's layout
#[derive(Debug, Clone, Copy)]
pub struct LimitBox {
  pub min_height: f32,
  pub max_height: f32,
  pub min_width: f32,
  pub max_width: f32,
}

pub const UNLIMIT_BOX: LimitBox = LimitBox {
  min_height: 0.0,
  max_height: f32::INFINITY,
  min_width: 0.0,
  max_width: f32::INFINITY,
};

pub const UNVALID_SIZE: Size = Size::new(-1.0, -1.0);

/// render object's layout box, the information about layout,
/// including box size, box position, and layout limit
#[derive(Debug)]
pub struct BoxLayout {
  /// box bound is the bound of the layout can be place. it should be set before
  /// render object's process of layout. when the object it is in the layout
  /// such as row, flex ... it's size is decided by his parent.
  pub limit: Option<LimitBox>,

  pub rect: Rect,
}

impl BoxLayout {
  pub fn new() -> BoxLayout {
    BoxLayout {
      limit: None,
      rect: Rect::new(Point::origin(), Size::new(-1.0, -1.0)),
    }
  }
}

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObjectSafety + Send + Sync>>,
  root: Option<RenderId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<RenderId, WidgetId>,
  /// Store the render object's place relative to parent coordinate after
  /// layout.
  box_place: HashMap<RenderId, BoxLayout>,

  /// root of sub tree which needed to perform layout
  dirty_layout_roots: HashSet<RenderId>,
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

  pub(crate) fn clean_layout_info(&mut self) {
    self.box_place.clear();
    self.dirty_layout_roots.clear();
  }

  pub(crate) fn drain_layout_roots(&mut self) -> Drain<'_, RenderId> {
    self.dirty_layout_roots.drain()
  }
}

impl RenderId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &RenderTree) -> Option<&(dyn RenderObjectSafety + Send + Sync)> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut<'a>(
    self,
    tree: &'a mut RenderTree,
  ) -> Option<&'a mut (dyn RenderObjectSafety + Send + Sync + 'static)> {
    tree.arena.get_mut(self.0).map(|node| &mut **node.get_mut())
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
  #[allow(dead_code)]
  pub(crate) fn children<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.children(&tree.arena).map(RenderId)
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
  #[allow(dead_code)]
  pub(crate) fn last_child(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  #[allow(dead_code)]
  pub(crate) fn previous_sibling(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub(crate) fn next_sibling(self, tree: &RenderTree) -> Option<RenderId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  #[allow(dead_code)]
  pub(crate) fn ancestors<'a>(self, tree: &'a RenderTree) -> impl Iterator<Item = RenderId> + 'a {
    self.0.ancestors(&tree.arena).map(RenderId)
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  #[allow(dead_code)]
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
    tree.box_place.remove(&self);
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  /// return the relative render widget.
  #[allow(dead_code)]
  pub(crate) fn relative_to_widget(self, tree: &mut RenderTree) -> Option<WidgetId> {
    tree.render_to_widget.get(&self).copied()
  }

  fn node_feature<F: Fn(&Node<Box<dyn RenderObjectSafety + Send + Sync>>) -> Option<NodeId>>(
    self,
    tree: &RenderTree,
    method: F,
  ) -> Option<RenderId> {
    tree.arena.get(self.0).map(method).flatten().map(RenderId)
  }

  /// return the render object placed position relative to its parent, this
  /// should only be called after layout, otherwise may return None or the place
  /// of last layout.
  pub(crate) fn box_rect(self, tree: &RenderTree) -> Option<&Rect> {
    tree.box_place.get(&self).map(|layout| &layout.rect)
  }

  pub(crate) fn update_position(self, tree: &mut RenderTree, pos: Point) {
    tree
      .box_place
      .entry(self)
      .or_insert(BoxLayout {
        limit: None,
        rect: Rect::zero(),
      })
      .rect
      .origin = pos;
  }

  pub(crate) fn update_size(self, tree: &mut RenderTree, size: Size) {
    tree
      .box_place
      .entry(self)
      .or_insert(BoxLayout::new())
      .rect
      .size = size;
  }

  pub(crate) fn set_box_limit(self, tree: &mut RenderTree, limit: Option<LimitBox>) {
    tree.box_place.entry(self).or_insert(BoxLayout::new()).limit = limit;
  }

  pub(crate) fn get_box_limit(self, tree: &RenderTree) -> Option<LimitBox> {
    tree.box_place.get(&self).and_then(|layout| layout.limit)
  }

  pub(crate) fn as_dirty_root(self, tree: &mut RenderTree) { tree.dirty_layout_roots.insert(self); }
}
