use crate::{prelude::*, render::render_tree::*, util::TreeFormatter};
use indextree::*;
use std::{
  collections::{HashMap, HashSet},
  pin::Pin,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
#[derive(Default)]
pub struct WidgetTree {
  arena: Arena<BoxWidget>,
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

  pub fn set_root(&mut self, data: BoxWidget, render_tree: &mut RenderTree) -> WidgetId {
    debug_assert!(self.root.is_none());
    let root = self.new_node(data);
    self.root = Some(root);
    self.inflate(root, render_tree);
    root
  }

  pub fn new_node(&mut self, widget: BoxWidget) -> WidgetId {
    if let Some(stateful) = widget.downcast_attr_widget::<stateful::StatefulAttr>() {
      let id = stateful.id();
      *id.get_mut(self).unwrap() = widget;
      id
    } else {
      self.alloc_node(widget)
    }
  }

  #[inline]
  pub fn alloc_node(&mut self, widget: BoxWidget) -> WidgetId {
    WidgetId(self.arena.new_node(widget))
  }

  /// inflate  subtree, so every subtree leaf should be a Widget::Render.
  pub fn inflate(&mut self, wid: WidgetId, render_tree: &mut RenderTree) -> &mut Self {
    let parent_id = wid
      .ancestors(self)
      .filter(|id| id.get(self).map_or(false, |w| w.is_render()))
      .find_map(|id| self.widget_to_render.get(&id))
      .copied();
    let mut stack = vec![(wid, parent_id)];

    while let Some((wid, parent_rid)) = stack.pop() {
      let (children, render) = {
        (
          wid.take_children(self),
          wid
            .get_mut(self)
            .and_then(|w| w.as_render())
            .map(|r| r.create_render_object()),
        )
      };

      let rid = render.map_or(parent_rid, |render_obj| {
        let rid = if let Some(id) = parent_rid {
          id.prepend_object(wid, render_obj, render_tree)
        } else {
          render_tree.set_root(wid, render_obj)
        };
        self.widget_to_render.insert(wid, rid);
        Some(rid)
      });

      if let Some(children) = children {
        children.into_iter().for_each(|w| {
          let id = wid.append_widget(w, self);
          stack.push((id, rid));
        });
      }
    }

    if let Some(rid) = parent_id.or_else(|| render_tree.root()) {
      rid.mark_needs_layout(render_tree);
    }

    self
  }

  /// Check all the need build widgets and update the widget tree to what need
  /// build widgets want it to be. Return if any node really rebuild or updated.
  pub fn repair(&mut self, render_tree: &mut RenderTree) -> bool {
    let repaired = !self.need_builds.is_empty() || !self.changed_widgets.is_empty();
    while let Some(need_build) = self.pop_need_build_widget() {
      debug_assert!(
        need_build.assert_get(self).is_combination(),
        "rebuild widget must be combination widget."
      );

      let mut stack = vec![need_build];

      while let Some(need_build) = stack.pop() {
        let children = need_build.take_children(self);

        if let Some(mut children) = children {
          if children.len() == 1 {
            let old_child_node = need_build.single_child(self);
            self.try_replace_widget_or_rebuild(
              old_child_node,
              children.pop().unwrap(),
              &mut stack,
              render_tree,
            );
          } else {
            self.repair_children_by_key(need_build, children, &mut stack, render_tree);
          }
        }
      }
    }

    self.flush_to_render(render_tree);
    repaired
  }

  #[cfg(test)]
  pub fn changed_widgets(&self) -> &HashSet<WidgetId> { &self.changed_widgets }

  #[cfg(test)]
  pub fn count(&self) -> usize { self.arena.count() }

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

      let safety = widget.as_render().expect("Must be a render widget!");

      rid
        .get_mut(r_tree1)
        .update(safety, &mut UpdateCtx::new(rid, r_tree2));
    });

    self.changed_widgets.clear();
  }

  /// Try to use `new_widget` to replace widget in old_node and push the
  /// `old_node` into stack, if they have same key. Other, drop the subtree.
  fn try_replace_widget_or_rebuild(
    &mut self,
    node: WidgetId,
    widget: BoxWidget,
    stack: &mut Vec<WidgetId>,
    render_tree: &mut RenderTree,
  ) {
    let same_key = widget
      .key()
      .and_then(|key| node.get(self).map(|w| Some(key) == w.key()))
      .unwrap_or(false);
    if same_key {
      if widget.is_render() {
        self.changed_widgets.insert(node);
      }
      *self
        .arena
        .get_mut(node.0)
        .expect("Widget not exist in the tree.")
        .get_mut() = widget;
      stack.push(node);
    } else {
      let parent_id = node.parent(&self).expect("parent should exists!");
      node.drop(self, render_tree);

      let new_child_id = parent_id.append_widget(widget, self);
      self.inflate(new_child_id, render_tree);
    }
  }

  /// rebuild the subtree `wid` by the new children `new_children`, the same key
  /// children as before will keep the old subtree and will add into the `stack`
  /// to recursive repair, else will construct a new subtree.
  fn repair_children_by_key(
    &mut self,
    node: WidgetId,
    new_children: SmallVec<[BoxWidget; 1]>,
    stack: &mut Vec<WidgetId>,
    render_tree: &mut RenderTree,
  ) {
    let mut key_children = HashMap::new();
    let mut child = node.first_child(self);
    while let Some(id) = child {
      child = id.next_sibling(self);

      let key = id.get(self).and_then(|w| w.key().cloned());
      if let Some(key) = key {
        id.detach(self);
        key_children.insert(key, id);
      } else {
        id.drop(self, render_tree);
      }
    }

    for w in new_children.into_iter() {
      if let Some(k) = w.key() {
        if let Some(id) = key_children.get(k).copied() {
          key_children.remove(k);
          node.0.append(id.0, &mut self.arena);
          self.try_replace_widget_or_rebuild(id, w, stack, render_tree);
          continue;
        }
      }

      let child_id = node.append_widget(w, self);
      self.inflate(child_id, render_tree);
    }

    key_children
      .into_iter()
      .for_each(|(_, v)| v.drop(self, render_tree));
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
  /// mark this id represented widget has changed, and need to update render
  /// tree in next frame.
  pub fn mark_changed(self, tree: &'_ mut WidgetTree) {
    if self.assert_get(tree).is_render() {
      tree.changed_widgets.insert(self);
    } else {
      tree.need_builds.insert(self);
    }
  }

  /// Returns a reference to the node data.
  pub fn get(self, tree: &WidgetTree) -> Option<&BoxWidget> {
    tree.arena.get(self.0).map(|node| node.get())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut BoxWidget> {
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
  pub fn ancestors<'a>(self, tree: &'a WidgetTree) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// A proxy for [NodeId::descendants](indextree::NodeId.descendants)

  pub fn descendants<'a>(self, tree: &'a WidgetTree) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  /// A proxy for [NodeId::detach](indextree::NodeId.detach)
  fn detach(self, tree: &mut WidgetTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  /// return the relative render widget.
  pub(crate) fn relative_to_render(self, tree: &WidgetTree) -> Option<RenderId> {
    let wid = self.down_nearest_render_widget(tree);
    tree.widget_to_render.get(&wid).cloned()
  }

  fn append_widget(self, data: BoxWidget, tree: &mut WidgetTree) -> WidgetId {
    let id = tree.new_node(data);
    self.0.append(id.0, &mut tree.arena);
    id
  }

  /// A proxy for [NodeId::remove](indextree::NodeId.remove)
  #[allow(dead_code)]
  pub(crate) fn remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena); }

  /// Drop the subtree
  fn drop(self, tree: &mut WidgetTree, render_tree: &mut RenderTree) {
    let rid = self.relative_to_render(tree).expect("must exists");
    let WidgetTree {
      widget_to_render,
      arena,
      changed_widgets,
      need_builds,
      ..
    } = tree;
    self.0.descendants(arena).map(WidgetId).for_each(|wid| {
      if arena
        .get(wid.0)
        .map_or(false, |node| node.get().is_render())
      {
        widget_to_render.remove(&wid);
      }
      changed_widgets.remove(&wid);
      need_builds.remove(&wid);
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

  /// find the nearest render widget in subtree, include self.
  fn down_nearest_render_widget(self, tree: &WidgetTree) -> WidgetId {
    let mut wid = self;
    while wid.get(tree).map_or(false, |w| w.is_combination()) {
      wid = wid.single_child(tree);
    }
    debug_assert!(wid.get(tree).map_or(false, |w| w.is_render()));
    wid
  }

  fn take_children(self, tree: &mut WidgetTree) -> Option<SmallVec<[BoxWidget; 1]>> {
    let (tree1, tree2) = unsafe {
      let ptr = tree as *mut WidgetTree;
      (&mut *ptr, &mut *ptr)
    };
    self.get_mut(tree1).and_then(|w| match w.classify_mut() {
      WidgetClassifyMut::Combination(c) => {
        let mut ctx = BuildCtx::new(unsafe { Pin::new_unchecked(tree2) }, self);
        Some(smallvec![c.build(&mut ctx)])
      }
      WidgetClassifyMut::Render(r) => r.take_children(),
    })
  }

  fn node_feature<F: Fn(&Node<BoxWidget>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub fn assert_get(self, tree: &WidgetTree) -> &BoxWidget {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut BoxWidget {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }
}

impl BoxWidget {
  fn key(&self) -> Option<&Key> { self.downcast_attr_widget::<Key>().map(|k| k.key()) }

  fn as_render(&self) -> Option<&dyn RenderWidgetSafety> {
    match self.classify() {
      WidgetClassify::Combination(_) => None,
      WidgetClassify::Render(r) => Some(r),
    }
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
  fn inflate_tree() {
    let (widget_tree, render_tree) = create_embed_app(3);

    assert_eq!(
      widget_tree.symbol_shape(),
      r#"EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 3 }
└── Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] })
    ├── Text("Simple demo")
    ├── Text("Adoo")
    ├── Text("Recursive x times")
    └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 2 }
        └── Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] })
            ├── Text("Simple demo")
            ├── Text("Adoo")
            ├── Text("Recursive x times")
            └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 1 }
                └── Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] })
                    ├── Text("Simple demo")
                    ├── Text("Adoo")
                    ├── Text("Recursive x times")
                    └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 0 }
                        └── Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] })
                            ├── Text("Simple demo")
                            ├── Text("Adoo")
                            └── Text("Recursive x times")
"#
    );

    assert_eq!(
      render_tree.symbol_shape(),
      r#"FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
├── TextRender { text: "Simple demo" }
├── TextRender { text: "Adoo" }
├── TextRender { text: "Recursive x times" }
└── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
    ├── TextRender { text: "Simple demo" }
    ├── TextRender { text: "Adoo" }
    ├── TextRender { text: "Recursive x times" }
    └── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
        ├── TextRender { text: "Simple demo" }
        ├── TextRender { text: "Adoo" }
        ├── TextRender { text: "Recursive x times" }
        └── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
            ├── TextRender { text: "Simple demo" }
            ├── TextRender { text: "Adoo" }
            └── TextRender { text: "Recursive x times" }
"#
    );
  }

  #[test]
  fn drop_all() {
    let (mut widget_tree, mut render_tree) = create_embed_app(3);

    widget_tree
      .root()
      .unwrap()
      .drop(&mut widget_tree, &mut render_tree);

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
  #[test]
  fn repair_tree() {
    let mut env = KeyDetectEnv::new(3);
    emit_rebuild(&mut env);
    env.widget_tree.repair(&mut env.render_tree);

    assert_eq!(
      env.widget_tree.symbol_shape(),
      r#"EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 3 }
└── WidgetAttr { attr: KI4(0), widget: Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] }), type_info: PhantomData }
    ├── WidgetAttr { attr: KI4(0), widget: Text("New title"), type_info: PhantomData }
    ├── WidgetAttr { attr: KI4(1), widget: Text(""), type_info: PhantomData }
    ├── WidgetAttr { attr: KI4(2), widget: Text(""), type_info: PhantomData }
    └── WidgetAttr { attr: KString("embed"), widget: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 2 }, type_info: PhantomData }
        └── WidgetAttr { attr: KI4(0), widget: Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] }), type_info: PhantomData }
            ├── WidgetAttr { attr: KI4(0), widget: Text("New title"), type_info: PhantomData }
            ├── WidgetAttr { attr: KI4(1), widget: Text(""), type_info: PhantomData }
            ├── WidgetAttr { attr: KI4(2), widget: Text(""), type_info: PhantomData }
            └── WidgetAttr { attr: KString("embed"), widget: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 1 }, type_info: PhantomData }
                └── WidgetAttr { attr: KI4(0), widget: Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] }), type_info: PhantomData }
                    ├── WidgetAttr { attr: KI4(0), widget: Text("New title"), type_info: PhantomData }
                    ├── WidgetAttr { attr: KI4(1), widget: Text(""), type_info: PhantomData }
                    ├── WidgetAttr { attr: KI4(2), widget: Text(""), type_info: PhantomData }
                    └── WidgetAttr { attr: KString("embed"), widget: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 0 }, type_info: PhantomData }
                        └── WidgetAttr { attr: KI4(0), widget: Row(Flex { reverse: false, wrap: false, direction: Horizontal, cross_align: Start, main_align: Start, children: [] }), type_info: PhantomData }
                            ├── WidgetAttr { attr: KI4(0), widget: Text("New title"), type_info: PhantomData }
                            ├── WidgetAttr { attr: KI4(1), widget: Text(""), type_info: PhantomData }
                            └── WidgetAttr { attr: KI4(2), widget: Text(""), type_info: PhantomData }
"#
    );

    assert_eq!(
      env.render_tree.symbol_shape(),
      r#"FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
├── TextRender { text: "New title" }
├── TextRender { text: "" }
├── TextRender { text: "" }
└── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
    ├── TextRender { text: "New title" }
    ├── TextRender { text: "" }
    ├── TextRender { text: "" }
    └── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
        ├── TextRender { text: "New title" }
        ├── TextRender { text: "" }
        ├── TextRender { text: "" }
        └── FlexRender { reverse: false, direction: Horizontal, cross_align: Start, main_align: Start, wrap: false }
            ├── TextRender { text: "New title" }
            ├── TextRender { text: "" }
            └── TextRender { text: "" }
"#
    );
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
