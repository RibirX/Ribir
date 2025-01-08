use std::rc::Rc;

use ahash::HashSetExt;
use indextree::{Node, NodeId};
use smallvec::{SmallVec, smallvec};

use super::*;
use crate::{
  data_widget::{AnonymousAttacher, DataAttacher},
  window::DelayEvent,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]

pub struct WidgetId(pub(crate) NodeId);

// A place holder get from a `WidgetId`, you can use it to insert a widget
// replace the `WidgetId` you get this place holder.
pub(crate) enum PlaceHolder {
  PrevSibling(WidgetId),
  NextSibling(WidgetId),
  Parent(WidgetId),
}

pub trait RenderQueryable: Render + Query {
  fn as_render(&self) -> &dyn Render;
  fn as_query(&self) -> &dyn Query;
}

impl<T: Render + Query> RenderQueryable for T {
  fn as_render(&self) -> &dyn Render { self }

  fn as_query(&self) -> &dyn Query { self }
}

/// You can get TrackId by builtin method of track_id().
/// see [`TrackWidgetId::track_id`]
pub struct TrackId(Stateful<Option<WidgetId>>);

impl TrackId {
  pub fn get(&self) -> Option<WidgetId> { *self.0.read() }

  pub fn watcher(&self) -> impl StateWatcher<Value = Option<WidgetId>> { self.0.clone_watcher() }

  pub(crate) fn set(&self, id: Option<WidgetId>) { *self.0.write() = id; }
}

impl dyn RenderQueryable {
  pub(crate) fn update_track_id(&self, new_id: WidgetId) {
    let mut handles = SmallVec::new();
    self.query_all(&QueryId::of::<TrackId>(), &mut handles);
    handles
      .into_iter()
      .filter_map(QueryHandle::into_ref::<TrackId>)
      .for_each(move |q| q.set(Some(new_id)));
  }
}

impl Default for TrackId {
  fn default() -> Self { Self(Stateful::new(None)) }
}

impl Clone for TrackId {
  fn clone(&self) -> Self { Self(self.0.clone_writer()) }
}

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get<'a, 'b>(self, tree: &'a WidgetTree) -> Option<&'a (dyn RenderQueryable + 'b)> {
    tree.arena.get(self.0).map(|n| &**n.get())
  }

  pub(crate) fn get_node_mut(self, tree: &mut WidgetTree) -> Option<&mut Box<dyn RenderQueryable>> {
    tree.arena.get_mut(self.0).map(|n| n.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &WidgetTree) -> bool { self.0.is_removed(&tree.arena) }

  pub(crate) fn lowest_common_ancestor(
    self, other: WidgetId, tree: &WidgetTree,
  ) -> Option<WidgetId> {
    self.common_ancestors(other, tree).last()
  }

  // return ancestors from root to lowest common ancestor
  pub(crate) fn common_ancestors(
    self, other: WidgetId, tree: &WidgetTree,
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

  pub(crate) fn place_holder(self, tree: &WidgetTree) -> PlaceHolder {
    if let Some(prev) = self.prev_sibling(tree) {
      PlaceHolder::PrevSibling(prev)
    } else if let Some(next) = self.next_sibling(tree) {
      PlaceHolder::NextSibling(next)
    } else {
      PlaceHolder::Parent(self.parent(tree).unwrap())
    }
  }

  fn delay_drop_parent(&self, tree: &WidgetTree) -> Option<WidgetId> {
    if self == &tree.root() {
      return None;
    }
    let wnd = tree.window();
    let delay_drop_widgets = wnd.delay_drop_widgets.borrow();
    delay_drop_widgets.iter().find_map(
      |(parent, id)| {
        if id.get() == Some(*self) { *parent } else { None }
      },
    )
  }

  pub(crate) fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self
      .node_feature(tree, |node| node.parent())
      .or_else(|| self.delay_drop_parent(tree))
  }

  pub(crate) fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  pub(crate) fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  pub(crate) fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }
  pub(crate) fn prev_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, Node::previous_sibling)
  }

  pub(crate) fn previous_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  #[allow(unused)]
  pub(crate) fn ancestor_of(self, other: WidgetId, tree: &WidgetTree) -> bool {
    other.ancestors(tree).any(|p| self == p)
  }

  pub(crate) fn ancestors(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    // `IndexTree` not check if is a freed id when create iterator, we may iterate
    // another node,so we need check it manually.
    assert!(!self.is_dropped(tree));

    let last_elem = Rc::new(RefCell::new(self));
    let last_elem2 = last_elem.clone();
    self
      .0
      .ancestors(&tree.arena)
      .map(move |id| {
        *last_elem2.borrow_mut() = WidgetId(id);
        WidgetId(id)
      })
      .chain(
        Some(last_elem)
          .into_iter()
          .filter_map(|v| v.borrow().delay_drop_parent(tree))
          .flat_map(|wid| wid.0.ancestors(&tree.arena).map(WidgetId)),
      )
  }

  #[inline]
  pub(crate) fn children(
    self, tree: &WidgetTree,
  ) -> impl DoubleEndedIterator<Item = WidgetId> + '_ {
    // `IndexTree` not check if is a freed id when create iterator, we may iterate
    assert!(!self.is_dropped(tree));
    self.0.children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn descendants(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    // another node,so we need check it manually.
    assert!(!self.is_dropped(tree));
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  pub(crate) fn on_widget_mounted(self, tree: &WidgetTree) {
    tree
      .window()
      .add_delay_event(DelayEvent::Mounted(self));
  }

  pub(crate) fn on_mounted_subtree(self, tree: &WidgetTree) {
    self.descendants(tree).for_each(|w| {
      tree
        .window()
        .add_delay_event(DelayEvent::Mounted(w))
    });
  }

  /// Dispose the whole subtree of `id`, include `id` itself.
  pub(crate) fn dispose_subtree(self, tree: &mut WidgetTree) {
    let parent = self.parent(tree);
    tree.detach(self);
    tree
      .window()
      .add_delay_event(DelayEvent::Disposed { id: self, parent });
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn insert_before(self, prev: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_before(prev.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  /// Traverses to the leaf widget in the widget tree, returning it. Panics if
  /// there is more than one child in the path.
  pub(crate) fn single_leaf(self, tree: &WidgetTree) -> WidgetId {
    let mut leaf = self;
    while let Some(child) = leaf.single_child(tree) {
      leaf = child;
    }
    leaf
  }
  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &WidgetTree) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree), "Have more than one child.");
    self.first_child(tree)
  }

  fn node_feature(
    self, tree: &WidgetTree, method: impl FnOnce(&Node<Box<dyn RenderQueryable>>) -> Option<NodeId>,
  ) -> Option<WidgetId> {
    tree
      .arena
      .get(self.0)
      .and_then(method)
      .map(WidgetId)
  }

  pub(crate) fn assert_get<'a, 'b>(self, tree: &'a WidgetTree) -> &'a (dyn RenderQueryable + 'b) {
    self
      .get(tree)
      .expect("Widget not exists in the `tree`")
  }

  /// We assume the `f` wrap the widget into a new widget, and keep the old
  /// widget as part of the new widget, otherwise, undefined behavior.
  pub(crate) fn wrap_node(
    self, tree: &mut WidgetTree,
    f: impl FnOnce(Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable>,
  ) {
    let node = self.get_node_mut(tree).unwrap();
    unsafe {
      let data = Box::from_raw(&mut **node as *mut _);
      let copied = std::mem::replace(node, f(data));
      std::mem::forget(copied)
    }
  }

  pub(crate) fn attach_data(self, data: Box<dyn Query>, tree: &mut WidgetTree) {
    self.wrap_node(tree, |node| Box::new(DataAttacher::new(node, data)));
  }

  pub(crate) fn attach_anonymous_data(self, data: impl Any, tree: &mut WidgetTree) {
    self.wrap_node(tree, |render| Box::new(AnonymousAttacher::new(render, Box::new(data))));
  }

  pub(crate) fn paint_subtree(self, ctx: &mut PaintingCtx) {
    let mut w = Some(self);

    let wnd = ctx.window();
    let tree = wnd.tree();

    let mut did_paint = HashSet::<WidgetId, ahash::RandomState>::new();
    while let Some(id) = w {
      ctx.id = id;

      let mut need_paint = false;
      if ctx.painter.alpha() != 0. {
        if let Some(layout_box) = ctx.box_rect() {
          let transform = *ctx.painter.transform();
          ctx
            .painter
            .translate(layout_box.min_x(), layout_box.min_y());
          if ctx
            .painter()
            .intersection_paint_bounds(&Rect::from_size(layout_box.size))
            .is_some()
          {
            let render = id.assert_get(tree);
            ctx.painter.set_transform(transform);
            ctx.painter.save();
            ctx
              .painter
              .translate(layout_box.min_x(), layout_box.min_y());
            render.paint(ctx);
            did_paint.insert(id);
            need_paint = true;
          } else {
            ctx.painter.set_transform(transform);
          }
        }
      }

      w = id
        .first_child(tree)
        .filter(|_| need_paint)
        .or_else(|| {
          let mut node = w;
          while let Some(p) = node {
            // self node sub-tree paint finished, goto sibling
            if did_paint.contains(&p) {
              did_paint.remove(&p);
              ctx.painter.restore();
            }
            node = p.next_sibling(tree);
            if node.is_some() {
              break;
            } else if p != self {
              // if there is no more sibling, back to parent to find sibling.
              node = p.parent(tree);
            }
          }
          node
        });
    }
  }
}

pub(crate) fn new_node(
  arena: &mut Arena<Box<dyn RenderQueryable>>, node: Box<dyn RenderQueryable>,
) -> WidgetId {
  WidgetId(arena.new_node(node))
}

impl WidgetId {
  /// Return a iterator of all reference of type `T` in this node.
  pub(crate) fn query_all_iter<T: Any>(
    self, tree: &WidgetTree,
  ) -> impl DoubleEndedIterator<Item = QueryRef<T>> {
    let mut out = smallvec![];
    self
      .assert_get(tree)
      .query_all(&QueryId::of::<T>(), &mut out);
    out.into_iter().filter_map(QueryHandle::into_ref)
  }

  #[allow(unused)]
  pub(crate) fn query_all_write_iter<T: Any>(
    self, tree: &WidgetTree,
  ) -> impl DoubleEndedIterator<Item = WriteRef<T>> {
    let mut out = smallvec![];
    self
      .assert_get(tree)
      .query_all_write(&QueryId::of::<T>(), &mut out);
    out.into_iter().filter_map(QueryHandle::into_mut)
  }

  #[allow(unused)]
  /// Query the outermost of reference of type `T` in this node.
  pub(crate) fn query_write<T: Any>(self, tree: &WidgetTree) -> Option<WriteRef<T>> {
    self
      .assert_get(tree)
      .query_write(&QueryId::of::<T>())
      .and_then(QueryHandle::into_mut)
  }

  /// Query the outermost of reference of type `T` in this node.
  pub(crate) fn query_ref<T: Any>(self, tree: &WidgetTree) -> Option<QueryRef<T>> {
    self
      .assert_get(tree)
      .query(&QueryId::of::<T>())
      .and_then(QueryHandle::into_ref)
  }

  pub(crate) fn query_ancestors_ref<T: Any>(self, tree: &WidgetTree) -> Option<QueryRef<T>> {
    self
      .ancestors(tree)
      .find_map(|id| id.query_ref::<T>(tree))
  }

  /// return if this object contain type `T`
  pub(crate) fn contain_type<T: Any>(self, tree: &WidgetTree) -> bool {
    self
      .assert_get(tree)
      .query(&QueryId::of::<T>())
      .is_some()
  }

  pub(crate) fn queryable(&self, tree: &WidgetTree) -> bool { self.assert_get(tree).queryable() }
}

impl PlaceHolder {
  pub(crate) fn replace(self, widget: WidgetId, tree: &mut WidgetTree) {
    match self {
      PlaceHolder::PrevSibling(prev) => prev.insert_after(widget, tree),
      PlaceHolder::NextSibling(next) => next.insert_before(widget, tree),
      PlaceHolder::Parent(parent) => parent.append(widget, tree),
    }
  }
}
