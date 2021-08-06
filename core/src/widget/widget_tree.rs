use crate::{prelude::*, render::render_tree::*};

use indextree::*;
use std::{
  collections::{HashMap, HashSet},
  pin::Pin,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
#[derive(Default)]
pub struct WidgetTree {
  arena: Arena<WidgetNode>,
  root: Option<WidgetId>,
  /// Store widgets that modified and wait to update its corresponds render
  /// object in render tree.
  changed_widgets: HashSet<WidgetId>,
  /// Store combination widgets that needs build subtree.
  need_builds: HashSet<WidgetId>,
  /// A hash map to mapping a render widget in widget tree to its corresponds
  /// render object in render tree.
  widget_to_render: HashMap<WidgetId, RenderId>,
}

impl WidgetTree {
  #[inline]
  pub fn root(&self) -> Option<WidgetId> { self.root }

  pub fn set_root(&mut self, root: BoxedWidget, r_tree: &mut RenderTree) -> WidgetId {
    struct Temp;
    impl CombinationWidget for Temp {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        unreachable!();
      }
    }

    debug_assert!(self.root.is_none());

    let temp = self.new_node(WidgetNode::Combination(Box::new(Temp)));
    let (root, _) = self.inflate(root, temp, r_tree);
    temp.0.remove(&mut self.arena);
    root.detach(self);
    self.root = Some(root);
    root
  }

  pub fn new_node(&mut self, widget: WidgetNode) -> WidgetId {
    let state_info = widget
      .get_attr()
      .and_then(|attrs| attrs.get::<StateAttr>())
      .cloned();
    let id = WidgetId(self.arena.new_node(widget));
    if let Some(state_info) = state_info {
      state_info.assign_id(id, std::ptr::NonNull::from(self))
    }
    id
  }

  /// inflate subtree, so every leaf should be a Widget::Render.
  pub fn inflate(
    &mut self,
    widget: BoxedWidget,
    parent: WidgetId,
    r_tree: &mut RenderTree,
  ) -> (WidgetId, RenderId) {
    let r_parent = parent
      .up_nearest_render_widget(self)
      .and_then(|wid| self.widget_to_render.get(&wid))
      .copied();

    let mut stack = vec![(widget, parent, r_parent)];

    while let Some((widget, p_wid, p_rid)) = stack.pop() {
      match widget {
        BoxedWidget::Combination(c) => {
          let c_ptr = &*c as *const dyn CombinationWidget;
          let wid = p_wid.append_widget(WidgetNode::Combination(c), self);
          let child = unsafe { &*c_ptr }.build_child(wid, self);
          stack.push((child, wid, p_rid));
        }
        BoxedWidget::Render(rw) => {
          self.append_render_widget(rw, p_wid, p_rid, r_tree);
        }
        BoxedWidget::SingleChild(s) => {
          let (rw, child) = s.unzip();
          let (wid, rid) = self.append_render_widget(rw, p_wid, p_rid, r_tree);
          stack.push((child, wid, Some(rid)));
        }
        BoxedWidget::MultiChild(m) => {
          let (rw, children) = m.unzip();
          let (wid, rid) = self.append_render_widget(rw, p_wid, p_rid, r_tree);
          children.into_iter().for_each(|w| {
            stack.push((w, wid, Some(rid)));
          });
        }
      };
    }

    // The root of inflated sub widget tree.
    let w_root = parent.last_child(self).unwrap();
    // The root of inflated sub render tree.
    let r_root = w_root
      .down_nearest_render_widget(self)
      .relative_to_render(self)
      .unwrap();
    if let Some(r_parent) = r_parent {
      r_parent.prepend(r_root, r_tree);
    } else {
      r_tree.set_root(r_root);
    }

    r_root.mark_needs_layout(r_tree);

    (w_root, r_root)
  }

  /// Check all the need build widgets and update the widget tree to what need
  /// build widgets want it to be. Return if any node really rebuild or updated.
  pub fn repair(&mut self, r_tree: &mut RenderTree) -> bool {
    let repaired = !self.need_builds.is_empty() || !self.changed_widgets.is_empty();
    while let Some(need_build) = self.pop_need_build_widget() {
      self.repair_subtree(need_build, r_tree)
    }

    self.flush_to_render(r_tree);
    repaired
  }

  #[cfg(test)]
  pub fn changed_widgets(&self) -> &HashSet<WidgetId> { &self.changed_widgets }

  #[cfg(test)]
  pub fn count(&self) -> usize { self.arena.count() }

  fn append_render_widget(
    &mut self,
    widget: Box<dyn RenderWidgetSafety>,
    p_wid: WidgetId,
    p_rid: Option<RenderId>,
    r_tree: &mut RenderTree,
  ) -> (WidgetId, RenderId) {
    let ro = widget.create_render_object();
    let wid = p_wid.append_widget(WidgetNode::Render(widget), self);
    let rid = r_tree.new_node(wid, ro);
    if let Some(p) = p_rid {
      p.prepend(rid, r_tree);
    }
    self.widget_to_render.insert(wid, rid);
    (wid, rid)
  }

  /// Tell the render object its owner changed one by one.
  fn flush_to_render(&mut self, render_tree: &mut RenderTree) {
    // Safety: just split render_tree as two to update render object, never modify
    // the render tree's struct.
    let (r_tree1, r_tree2) = unsafe {
      let ptr = render_tree as *mut RenderTree;
      (&mut *ptr, &mut *ptr)
    };
    self.changed_widgets.iter().for_each(|wid| {
      let widget = wid.assert_get(self);

      let rid = *self
        .widget_to_render
        .get(wid)
        .expect("Changed widget should always render widget!");

      let safety = match widget {
        WidgetNode::Combination(_) => unreachable!("Must be a render widget!"),
        WidgetNode::Render(r) => r,
      };

      rid.get_mut(r_tree1).update(
        safety.clone_boxed_states(),
        &mut UpdateCtx::new(rid, r_tree2),
      );
    });

    self.changed_widgets.clear();
  }

  fn repair_subtree(&mut self, sub_tree: WidgetId, r_tree: &mut RenderTree) {
    let c = match sub_tree.assert_get(self) {
      WidgetNode::Combination(c) => c,
      WidgetNode::Render(_) => unreachable!("rebuild widget must be combination widget."),
    };

    let child = c.build_child(sub_tree, self);
    let child_id = sub_tree.single_child(self);

    let mut stack = vec![(child, child_id)];
    while let Some((w, wid)) = stack.pop() {
      match w {
        BoxedWidget::Combination(c) => {
          let (new_id, child) = self.update_combination_widget_by_diff(c, wid, r_tree);
          self.need_builds.remove(&wid);
          if new_id == wid {
            stack.push((child, wid.single_child(self)));
          } else {
            self.inflate(child, new_id, r_tree);
          }
        }
        BoxedWidget::Render(r) => {
          self.update_render_widget_by_diff(r, wid, r_tree);
        }
        BoxedWidget::SingleChild(s) => {
          let (r, child) = s.unzip();
          let new_id = self.update_render_widget_by_diff(r, wid, r_tree);
          if new_id == wid {
            stack.push((child, wid.single_child(self)));
          } else {
            self.inflate(child, new_id, r_tree);
          }
        }
        BoxedWidget::MultiChild(m) => {
          let (r, children) = m.unzip();
          let new_id = self.update_render_widget_by_diff(r, wid, r_tree);
          if new_id == wid {
            let mut key_children = self.collect_key_children(wid, r_tree);
            children.into_iter().for_each(|c| {
              let k_widget = c.key().and_then(|k| key_children.remove(k));
              if let Some(id) = k_widget {
                new_id.0.append(id.0, &mut self.arena);
                stack.push((c, id));
              } else {
                self.inflate(c, new_id, r_tree);
              }
            });
            key_children
              .into_iter()
              .for_each(|(_, v)| v.drop_subtree(self, r_tree));
          } else {
            children.into_iter().for_each(|c| {
              self.inflate(c, new_id, r_tree);
            })
          }
        }
      }
    }
  }

  fn update_combination_widget_by_diff(
    &mut self,
    c: Box<dyn CombinationWidget>,
    id: WidgetId,
    r_tree: &mut RenderTree,
  ) -> (WidgetId, BoxedWidget) {
    let c_ptr = &*c as *const dyn CombinationWidget;
    let new_id = self.update_widget_by_diff(WidgetNode::Combination(c), id, r_tree);

    let child = unsafe { &*c_ptr }.build_child(id, self);

    (new_id, child)
  }

  fn update_render_widget_by_diff(
    &mut self,
    r: Box<dyn RenderWidgetSafety>,
    id: WidgetId,
    r_tree: &mut RenderTree,
  ) -> WidgetId {
    let new_id = self.update_widget_by_diff(WidgetNode::Render(r), id, r_tree);
    if id != new_id {
      self.changed_widgets.insert(id);
    }
    new_id
  }

  // update widget by key diff, return the new id of the widget.
  fn update_widget_by_diff(
    &mut self,
    w: WidgetNode,
    id: WidgetId,
    r_tree: &mut RenderTree,
  ) -> WidgetId {
    let old = id.assert_get_mut(self);
    let o_key = old.key();
    if o_key.is_some() && o_key != w.key() {
      *old = w;
      id
    } else {
      let parent = id.parent(&self).expect("parent should exists!");
      id.drop_subtree(self, r_tree);
      parent.append_widget(w, self)
    }
  }

  // Collect and detach the child has key, and drop the others.
  fn collect_key_children(
    &mut self,
    wid: WidgetId,
    r_tree: &mut RenderTree,
  ) -> HashMap<Key, WidgetId> {
    let mut key_children = HashMap::new();
    let mut child = wid.first_child(self);
    while let Some(id) = child {
      child = id.next_sibling(self);

      let key = id.get(self).and_then(|w| w.key().cloned());
      if let Some(key) = key {
        id.detach(self);
        key_children.insert(key, id);
      } else {
        id.drop_subtree(self, r_tree);
      }
    }
    key_children
  }

  /// Return the topmost need rebuild
  fn pop_need_build_widget(&mut self) -> Option<WidgetId> {
    let topmost = self
      .need_builds
      .iter()
      .next()
      .and_then(|id| id.ancestors(self).find(|id| self.need_builds.contains(id)));

    if let Some(topmost) = topmost.as_ref() {
      self.need_builds.remove(topmost);
    }
    topmost
  }
}

impl WidgetId {
  /// mark this id represented widget has changed, and need to update render
  /// tree in next frame.
  pub fn mark_changed(self, tree: &'_ mut WidgetTree) {
    if matches!(self.assert_get(tree), WidgetNode::Render(_)) {
      tree.changed_widgets.insert(self);
    } else {
      tree.need_builds.insert(self);
    }
  }

  /// Returns a reference to the node data.
  pub fn get(self, tree: &WidgetTree) -> Option<&WidgetNode> {
    tree.arena.get(self.0).map(|node| node.get())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut WidgetNode> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub fn is_dropped(self, tree: &WidgetTree) -> bool { self.0.is_removed(&tree.arena) }

  #[allow(clippy::needless_collect)]
  pub fn common_ancestor_of(self, other: WidgetId, tree: &WidgetTree) -> Option<WidgetId> {
    if self.is_dropped(tree) || other.is_dropped(tree) {
      return None;
    }

    let other_path = other.ancestors(tree).collect::<Vec<_>>();
    let self_path = self.ancestors(tree).collect::<Vec<_>>();

    let min_len = other_path.len().min(self_path.len());
    (1..=min_len)
      .find(|idx| other_path[other_path.len() - idx] != self_path[self_path.len() - idx])
      // if one widget is the ancestor of the other, the reverse index `min_len` store the common
      // ancestor.
      .or(Some(min_len + 1))
      .and_then(|r_idx| {
        let idx = self_path.len() + 1 - r_idx;
        self_path.get(idx).cloned()
      })
  }

  /// A proxy for [NodeId::parent](indextree::NodeId.parent)
  pub fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  /// A proxy for [NodeId::first_child](indextree::NodeId.first_child)
  pub fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  /// A proxy for [NodeId::last_child](indextree::NodeId.last_child)
  pub fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  /// A proxy for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub fn previous_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  /// A proxy for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  /// A proxy for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub fn ancestors(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// A proxy for [NodeId::descendants](indextree::NodeId.descendants)

  pub fn descendants(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  /// return the relative render widget.
  pub(crate) fn relative_to_render(self, tree: &WidgetTree) -> Option<RenderId> {
    let wid = self.down_nearest_render_widget(tree);
    tree.widget_to_render.get(&wid).cloned()
  }

  /// A proxy for [NodeId::remove](indextree::NodeId.remove)
  #[allow(dead_code)]
  pub(crate) fn remove(self, tree: &mut WidgetTree) {
    self.clear_info(tree);

    self.0.remove(&mut tree.arena);
  }

  /// A proxy for [NodeId::detach](indextree::NodeId.detach)
  fn detach(self, tree: &mut WidgetTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  fn append_widget(self, data: WidgetNode, tree: &mut WidgetTree) -> WidgetId {
    let id = tree.new_node(data);
    self.0.append(id.0, &mut tree.arena);
    id
  }

  /// Drop the subtree
  fn drop_subtree(self, tree: &mut WidgetTree, render_tree: &mut RenderTree) {
    let rid = self.relative_to_render(tree).expect("must exists");
    // split tree
    let (tree1, tree2) = unsafe {
      let ptr = tree as *mut WidgetTree;
      (&mut *ptr, &mut *ptr)
    };
    self.descendants(tree1).for_each(|wid| {
      wid.clear_info(tree2);
    });

    rid.drop(render_tree);
    self.0.remove_subtree(&mut tree.arena);
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  /// Caller assert this node only have one child, other panic!
  fn single_child(self, tree: &WidgetTree) -> WidgetId {
    debug_assert!(self.first_child(tree).is_some());
    debug_assert_eq!(self.first_child(tree), self.last_child(tree));
    self
      .first_child(tree)
      .expect("Caller assert `wid` has single child")
  }

  /// find the nearest render widget of its ancestors.
  fn up_nearest_render_widget(self, tree: &WidgetTree) -> Option<WidgetId> {
    self
      .ancestors(tree)
      .filter(|id| matches!(id.get(tree), Some(WidgetNode::Render(_))))
      .next()
  }

  /// find the nearest render widget in subtree, include self.
  fn down_nearest_render_widget(self, tree: &WidgetTree) -> WidgetId {
    let mut wid = self;
    while matches!(wid.assert_get(tree), WidgetNode::Combination(_)) {
      wid = wid.single_child(tree);
    }

    debug_assert!(matches!(wid.assert_get(tree), WidgetNode::Render(_)));
    wid
  }

  fn node_feature<F: Fn(&Node<WidgetNode>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  fn clear_info(self, tree: &mut WidgetTree) {
    if matches!(self.get(tree), Some(WidgetNode::Render(_))) {
      tree.widget_to_render.remove(&self);
    }
    tree.changed_widgets.remove(&self);
    tree.need_builds.remove(&self);
  }

  pub fn assert_get(self, tree: &WidgetTree) -> &WidgetNode {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut WidgetNode {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }
}

impl !Unpin for WidgetTree {}

impl WidgetId {
  /// Return a dummy `WidgetId` use for unit test.
  /// # Safety
  /// Just use it for unit test or ensure you will reassign a valid WidgetId
  /// from `WidgetTree`
  pub unsafe fn dummy() -> Self {
    let index = std::num::NonZeroUsize::new(0);
    std::mem::transmute((index, 0))
  }
}

impl dyn CombinationWidget {
  fn build_child(&self, wid: WidgetId, tree: &WidgetTree) -> BoxedWidget {
    let c_ptr = self as *const dyn CombinationWidget;
    let mut ctx = BuildCtx::new(unsafe { Pin::new_unchecked(tree) }, wid);
    unsafe { &*c_ptr }.build(&mut ctx)
  }
}

pub enum WidgetNode {
  Combination(Box<dyn CombinationWidget>),
  Render(Box<dyn RenderWidgetSafety>),
}

impl WidgetNode {
  pub fn key(&self) -> Option<&Key> { self.find_attr() }

  /// Find an attr of this widget. If it have the `A` type attr, return the
  /// reference.
  pub fn find_attr<A: Any>(&self) -> Option<&A> { self.get_attr().and_then(|attr| attr.get()) }

  fn get_attr(&self) -> Option<&Attributes> {
    match self {
      WidgetNode::Combination(c) => c.get_attrs(),
      WidgetNode::Render(r) => r.get_attrs(),
    }
  }
}

impl BoxedWidget {
  fn key(&self) -> Option<&Key> {
    let attrs = match self {
      BoxedWidget::Combination(c) => c.get_attrs(),
      BoxedWidget::Render(r) => r.get_attrs(),
      BoxedWidget::SingleChild(s) => s.get_attrs(),
      BoxedWidget::MultiChild(m) => m.get_attrs(),
    };
    attrs.and_then(|attrs| attrs.get())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::test::{
    embed_post::{create_embed_app, EmbedPost},
    recursive_row::RecursiveRow,
  };

  extern crate test;
  use test::Bencher;

  fn create_env(level: usize) -> WidgetTree {
    let mut tree = WidgetTree::default();
    let mut render_tree = RenderTree::default();
    tree.set_root(EmbedPost::new(level).box_it(), &mut render_tree);
    tree
  }

  #[test]
  fn drop_all() {
    let (mut widget_tree, mut render_tree) = create_embed_app(3);

    widget_tree
      .root()
      .unwrap()
      .drop_subtree(&mut widget_tree, &mut render_tree);

    assert!(widget_tree.widget_to_render.is_empty());
    assert!(render_tree.render_to_widget().is_empty());
    assert!(widget_tree.need_builds.is_empty());
    assert!(widget_tree.changed_widgets.is_empty());
    assert!(widget_tree.root().is_none());
    assert!(render_tree.root().is_none());
  }

  use crate::test::key_embed_post::KeyDetectEnv;

  fn emit_rebuild(env: &mut KeyDetectEnv) {
    *env.title.borrow_mut() = "New title";
    env
      .widget_tree
      .need_builds
      .insert(env.widget_tree.root().unwrap());
  }

  fn test_sample_create(width: usize, depth: usize) -> (WidgetTree, RenderTree) {
    let mut widget_tree = WidgetTree::default();
    let mut render_tree = RenderTree::default();
    let root = RecursiveRow { width, depth };
    widget_tree.set_root(root.box_it(), &mut render_tree);
    (widget_tree, render_tree)
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) { b.iter(|| create_env(1000)); }

  #[bench]
  fn inflate_50_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(50, 2)) }

  #[bench]
  fn inflate_100_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(100, 2)) }

  #[bench]
  fn inflate_10_pow_4(b: &mut Bencher) { b.iter(|| test_sample_create(10, 4)) }

  #[bench]
  fn inflate_10_pow_5(b: &mut Bencher) { b.iter(|| test_sample_create(10, 5)) }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let mut env = KeyDetectEnv::new(1000);
    b.iter(|| {
      emit_rebuild(&mut env);
      env.widget_tree.repair(&mut env.render_tree);
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let (mut widget_tree, mut render_tree) = test_sample_create(50, 2);
    b.iter(|| {
      widget_tree.need_builds.insert(widget_tree.root().unwrap());
      widget_tree.repair(&mut render_tree)
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let (mut widget_tree, mut render_tree) = test_sample_create(100, 2);
    b.iter(|| {
      widget_tree.need_builds.insert(widget_tree.root().unwrap());
      widget_tree.repair(&mut render_tree)
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let (mut widget_tree, mut render_tree) = test_sample_create(10, 4);
    b.iter(|| {
      widget_tree.need_builds.insert(widget_tree.root().unwrap());
      widget_tree.repair(&mut render_tree)
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let (mut widget_tree, mut render_tree) = test_sample_create(10, 5);
    b.iter(|| {
      widget_tree.need_builds.insert(widget_tree.root().unwrap());
      widget_tree.repair(&mut render_tree)
    })
  }
}
