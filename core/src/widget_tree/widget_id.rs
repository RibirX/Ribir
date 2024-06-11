use std::any::{Any, TypeId};

use indextree::{Arena, Node, NodeId};

use super::{Query, QueryHandle, QueryRef, WidgetTree, WriteRef};
use crate::{
  context::{PaintingCtx, WidgetCtx},
  data_widget::{AnonymousAttacher, DataAttacher},
  widget::Render,
  window::DelayEvent,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]

pub struct WidgetId(pub(crate) NodeId);

pub trait RenderQueryable: Render + Query {}

impl<T: Render + Query> RenderQueryable for T {}

pub(crate) type TreeArena = Arena<Box<dyn RenderQueryable>>;

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get<'a, 'b>(self, tree: &'a TreeArena) -> Option<&'a (dyn RenderQueryable + 'b)> {
    tree.get(self.0).map(|n| &**n.get())
  }

  pub(crate) fn get_node_mut(self, tree: &mut TreeArena) -> Option<&mut Box<dyn RenderQueryable>> {
    tree.get_mut(self.0).map(|n| n.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &TreeArena) -> bool { self.0.is_removed(tree) }

  pub(crate) fn lowest_common_ancestor(
    self, other: WidgetId, tree: &TreeArena,
  ) -> Option<WidgetId> {
    self.common_ancestors(other, tree).last()
  }

  // return ancestors from root to lowest common ancestor
  pub(crate) fn common_ancestors(
    self, other: WidgetId, tree: &TreeArena,
  ) -> impl Iterator<Item = WidgetId> + '_ {
    let mut p0 = vec![];
    let mut p1 = vec![];
    if !self.is_dropped(tree) && !other.is_dropped(tree) {
      p0 = other.ancestors(tree).collect::<Vec<_>>();
      p1 = self.ancestors(tree).collect::<Vec<_>>();
    }

    p0.into_iter()
      .rev()
      .zip(p1.into_iter().rev())
      .take_while(|(a, b)| a == b)
      .map(|(a, _)| a)
  }

  pub(crate) fn parent(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  pub(crate) fn first_child(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  pub(crate) fn last_child(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  pub(crate) fn next_sibling(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }
  pub(crate) fn prev_sibling(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, Node::previous_sibling)
  }

  pub(crate) fn previous_sibling(self, tree: &TreeArena) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  pub fn ancestor_of(self, other: WidgetId, tree: &TreeArena) -> bool {
    other.ancestors(tree).any(|p| self == p)
  }

  pub(crate) fn ancestors(self, tree: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    // `IndexTree` not check if is a freed id when create iterator, we may iterate
    // another node,so we need check it manually.
    assert!(!self.is_dropped(tree));
    self.0.ancestors(tree).map(WidgetId)
  }

  #[inline]
  pub(crate) fn children(self, arena: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    // `IndexTree` not check if is a freed id when create iterator, we may iterate
    assert!(!self.is_dropped(arena));
    self.0.children(arena).map(WidgetId)
  }

  pub(crate) fn descendants(self, tree: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    // another node,so we need check it manually.
    assert!(!self.is_dropped(tree));
    self.0.descendants(tree).map(WidgetId)
  }

  pub(crate) fn on_mounted_subtree(self, tree: &WidgetTree) {
    self.descendants(&tree.arena).for_each(|w| {
      tree
        .window()
        .add_delay_event(DelayEvent::Mounted(w))
    });
  }

  /// Dispose the whole subtree of `id`, include `id` itself.
  pub(crate) fn dispose_subtree(self, tree: &mut WidgetTree) {
    let parent = self.parent(&tree.arena);
    tree.detach(self);
    tree
      .window()
      .add_delay_event(DelayEvent::Disposed { id: self, parent });
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut TreeArena) {
    self.0.insert_after(next.0, tree);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut TreeArena) {
    self.0.append(child.0, tree);
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &TreeArena) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree), "Have more than one child.");
    self.first_child(tree)
  }

  fn node_feature(
    self, tree: &TreeArena, method: impl FnOnce(&Node<Box<dyn RenderQueryable>>) -> Option<NodeId>,
  ) -> Option<WidgetId> {
    tree.get(self.0).and_then(method).map(WidgetId)
  }

  pub(crate) fn assert_get<'a, 'b>(self, tree: &'a TreeArena) -> &'a (dyn RenderQueryable + 'b) {
    self
      .get(tree)
      .expect("Widget not exists in the `tree`")
  }

  /// We assume the `f` wrap the widget into a new widget, and keep the old
  /// widget as part of the new widget, otherwise, undefined behavior.
  pub(crate) fn wrap_node(
    self, tree: &mut TreeArena,
    f: impl FnOnce(Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable>,
  ) {
    let node = self.get_node_mut(tree).unwrap();
    unsafe {
      let data = Box::from_raw(&mut **node as *mut _);
      let copied = std::mem::replace(node, f(data));
      std::mem::forget(copied)
    }
  }

  pub(crate) fn attach_data(self, data: impl Query, tree: &mut TreeArena) {
    self.wrap_node(tree, |node| Box::new(DataAttacher::new(node, data)));
  }

  pub(crate) fn attach_anonymous_data(self, data: impl Any, tree: &mut TreeArena) {
    self.wrap_node(tree, |render| Box::new(AnonymousAttacher::new(render, Box::new(data))));
  }

  pub(crate) fn paint_subtree(self, ctx: &mut PaintingCtx) {
    let mut w = Some(self);
    while let Some(id) = w {
      ctx.id = id;
      ctx.painter.save();
      let wnd = ctx.window();
      let arena = &wnd.widget_tree.borrow().arena;

      let mut need_paint = false;
      if ctx.painter.alpha() != 0. {
        if let Some(layout_box) = ctx.box_rect() {
          let render = id.assert_get(arena);
          ctx
            .painter
            .translate(layout_box.min_x(), layout_box.min_y());
          render.paint(ctx);
          need_paint = true;
        }
      }

      w = id
        .first_child(arena)
        .filter(|_| need_paint)
        .or_else(|| {
          let mut node = w;
          while let Some(p) = node {
            // self node sub-tree paint finished, goto sibling
            ctx.painter.restore();
            node = match p == self {
              true => None,
              false => p.next_sibling(arena),
            };
            if node.is_some() {
              break;
            } else {
              // if there is no more sibling, back to parent to find sibling.
              node = p.parent(arena);
            }
          }
          node
        });
    }
  }
}

pub(crate) fn new_node(arena: &mut TreeArena, node: Box<dyn RenderQueryable>) -> WidgetId {
  WidgetId(arena.new_node(node))
}

impl<'a> dyn RenderQueryable + 'a {
  /// Return a iterator of all reference of type `T` in this node.
  pub fn query_all_iter<T: Any>(&self) -> impl DoubleEndedIterator<Item = QueryRef<T>> {
    self
      .query_all(TypeId::of::<T>())
      .into_iter()
      .filter_map(QueryHandle::into_ref)
  }

  /// Return a iterator of all mutable reference of type `T` in this node.
  pub fn query_all_iter_write<T: Any>(&self) -> impl DoubleEndedIterator<Item = WriteRef<T>> {
    self
      .query_all(TypeId::of::<T>())
      .into_iter()
      .filter_map(QueryHandle::into_mut)
  }

  /// Query the outermost of reference of type `T` in this node.
  pub fn query_write<T: Any>(&self) -> Option<WriteRef<T>> {
    self
      .query(TypeId::of::<T>())
      .and_then(QueryHandle::into_mut)
  }

  /// Query the outermost of reference of type `T` in this node.
  pub fn query_ref<T: Any>(&self) -> Option<QueryRef<T>> {
    self
      .query(TypeId::of::<T>())
      .and_then(QueryHandle::into_ref)
  }

  /// return if this object contain type `T`
  pub fn contain_type<T: Any>(&self) -> bool { self.query(TypeId::of::<T>()).is_some() }
}
