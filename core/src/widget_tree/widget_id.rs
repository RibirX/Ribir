use indextree::{Arena, Node, NodeId};
use rxrust::prelude::*;

use crate::{
  builtin_widgets::{DisposedListener, MountedListener, Void},
  context::{LifeCycleCtx, WindowCtx},
  widget::{ModifyScope, QueryOrder, Render, StateChangeNotifier},
};

use super::{DirtySet, LayoutStore};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]

pub struct WidgetId(pub(crate) NodeId);

pub(crate) type TreeArena = Arena<Box<dyn Render>>;

impl WidgetId {
  pub(crate) fn new_node(tree: &mut TreeArena) -> WidgetId {
    WidgetId(tree.new_node(Box::new(Void)))
  }

  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &TreeArena) -> Option<&dyn Render> {
    tree.get(self.0).map(|node| node.get().as_ref())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut TreeArena) -> Option<&mut Box<dyn Render>> {
    tree.get_mut(self.0).map(|node| node.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &TreeArena) -> bool { self.0.is_removed(&tree) }

  #[allow(clippy::needless_collect)]
  pub(crate) fn lowest_common_ancestor(
    self,
    other: WidgetId,
    tree: &TreeArena,
  ) -> Option<WidgetId> {
    self.common_ancestors(other, tree).last()
  }

  #[allow(clippy::needless_collect)]
  // return ancestors from root to lowest common ancestor
  pub(crate) fn common_ancestors(
    self,
    other: WidgetId,
    tree: &TreeArena,
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

  pub(crate) fn ancestors(self, tree: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.ancestors(tree).map(WidgetId)
  }

  /// Detect if this widget is the ancestors of `w`
  pub(crate) fn ancestors_of(self, w: WidgetId, tree: &TreeArena) -> bool {
    w.ancestors(tree).any(|a| a == self)
  }

  #[inline]
  pub(crate) fn children(self, arena: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.children(arena).map(WidgetId)
  }

  pub(crate) fn reverse_children(self, arena: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.reverse_children(arena).map(WidgetId)
  }

  pub(crate) fn descendants(self, tree: &TreeArena) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.descendants(tree).map(WidgetId)
  }

  pub(crate) fn swap_id(self, other: WidgetId, tree: &mut TreeArena) {
    self.swap_data(other, tree);

    let guard = WidgetId::new_node(tree);
    self.transplant(guard, tree);
    other.transplant(self, tree);
    guard.transplant(other, tree);
    guard.0.remove(tree);
  }

  pub(crate) fn transplant(self, other: WidgetId, tree: &mut TreeArena) {
    self.insert_after(other, tree);
    let first_child = self.first_child(tree);
    let mut cursor = first_child;
    while let Some(c) = cursor {
      cursor = c.next_sibling(tree);
      other.append(c, tree);
    }
    self.detach(tree);
  }

  pub(crate) fn swap_data(self, other: WidgetId, tree: &mut TreeArena) {
    // Safety: mut borrow two node not intersect.
    let (tree1, tree2) = unsafe { split_arena(tree) };
    std::mem::swap(self.assert_get_mut(tree1), other.assert_get_mut(tree2));
  }

  pub(crate) fn detach(self, tree: &mut TreeArena) { self.0.detach(tree) }

  pub(crate) fn remove_subtree(
    self,
    arena: &mut TreeArena,
    store: &mut LayoutStore,
    wnd_ctx: &WindowCtx,
  ) {
    self.descendants(arena).for_each(|id| {
      store.remove(id);

      id.assert_get(arena).query_all_type(
        |d: &DisposedListener| {
          (d.disposed.borrow_mut())(LifeCycleCtx { id: self, arena, store, wnd_ctx });
          true
        },
        QueryOrder::OutsideFirst,
      )
    });

    self.0.remove_subtree(arena)
  }

  pub(crate) fn on_mounted_subtree(
    self,
    arena: &TreeArena,
    store: &LayoutStore,
    wnd_ctx: &WindowCtx,
    dirty_set: &DirtySet,
  ) {
    self
      .descendants(arena)
      .for_each(|w| w.on_mounted(arena, store, wnd_ctx, dirty_set));
  }

  pub(crate) fn on_mounted(
    self,
    arena: &TreeArena,
    store: &LayoutStore,
    wnd_ctx: &WindowCtx,
    dirty_sets: &DirtySet,
  ) {
    self.assert_get(arena).query_all_type(
      |notifier: &StateChangeNotifier| {
        let state_changed = dirty_sets.clone();
        notifier
          .raw_modifies()
          .filter(|b| b.contains(ModifyScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(self);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    self.assert_get(arena).query_all_type(
      |m: &MountedListener| {
        (m.mounted.borrow_mut())(LifeCycleCtx { id: self, arena, store, wnd_ctx });
        true
      },
      QueryOrder::OutsideFirst,
    );
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut TreeArena) {
    self.0.insert_after(next.0, tree);
  }

  pub(crate) fn insert_before(self, before: WidgetId, tree: &mut TreeArena) {
    self.0.insert_before(before.0, tree);
  }

  pub(crate) fn prepend(self, child: WidgetId, tree: &mut TreeArena) {
    self.0.prepend(child.0, tree);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut TreeArena) {
    self.0.append(child.0, tree);
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &TreeArena) -> Option<WidgetId> {
    assert_eq!(
      self.first_child(tree),
      self.last_child(tree),
      "Have more than one child."
    );
    self.first_child(tree)
  }

  fn node_feature<F: Fn(&Node<Box<dyn Render>>) -> Option<NodeId>>(
    self,
    tree: &TreeArena,
    method: F,
  ) -> Option<WidgetId> {
    tree.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub(crate) fn assert_get(self, tree: &TreeArena) -> &dyn Render {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn assert_get_mut(self, tree: &mut TreeArena) -> &mut Box<dyn Render> {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }
}

pub(crate) unsafe fn split_arena(tree: &mut TreeArena) -> (&mut TreeArena, &mut TreeArena) {
  let ptr = tree as *mut TreeArena;
  (&mut *ptr, &mut *ptr)
}

pub(crate) fn new_node(arena: &mut TreeArena, node: Box<dyn Render>) -> WidgetId {
  WidgetId(arena.new_node(node))
}

pub(crate) fn empty_node(arena: &mut TreeArena) -> WidgetId { new_node(arena, Box::new(Void)) }
