use crate::{prelude::*, render::render_tree::*, util::TreeFormatter};
use indextree::*;
use std::{
  cell::{Ref, RefCell, RefMut},
  collections::{HashMap, HashSet},
  rc::Rc,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
#[derive(Default)]
pub struct WidgetTree {
  arena: Arena<WidgetNode>,
  root: Option<WidgetId>,
  // todo: merge changed_widgets and need_builds
  /// Store widgets that modified and wait to update its corresponds render
  /// object in render tree.
  pub(crate) changed_widgets: HashSet<WidgetId>,
  /// Store combination widgets that needs build subtree.
  pub(crate) need_builds: HashSet<WidgetId>,
  /// A hash map to mapping a render widget in widget tree to its corresponds
  /// render object in render tree.
  widget_to_render: HashMap<WidgetId, RenderId>,
}

impl WidgetTree {
  #[inline]
  pub fn root(&self) -> Option<WidgetId> { self.root }

  pub fn set_root(&mut self, data: Box<dyn Widget>, render_tree: &mut RenderTree) -> WidgetId {
    debug_assert!(self.root.is_none());
    let root = self.new_node(data);
    self.root = Some(root);
    self.inflate(root, render_tree);
    root
  }

  #[inline]
  pub fn new_node<N: Into<WidgetNode>>(&mut self, data: N) -> WidgetId {
    let data = data.into();

    // stateful widget is a preallocate node, should not allocate again.
    let id = {
      let widget = data.borrow();
      widget
        .downcast_ref::<super::stateful::StatefulWidget>()
        .map(|stateful| stateful.id())
    };
    id.unwrap_or_else(|| WidgetId(self.arena.new_node(data)))
  }

  /// inflate  subtree, so every subtree leaf should be a Widget::Render.
  pub fn inflate(&mut self, wid: WidgetId, render_tree: &mut RenderTree) -> &mut Self {
    let parent_id = wid
      .ancestors(self)
      .find(|id| id.get(self).map_or(false, |w| w.classify().is_render()))
      .and_then(|id| self.widget_to_render.get(&id))
      .copied();
    let mut stack = vec![(wid, parent_id)];

    while let Some((wid, parent_rid)) = stack.pop() {
      let (children, render) = {
        let mut widget = wid.get_mut(self).expect("must exist!");
        (
          widget.take_children(),
          widget.as_render().map(|r| r.create_render_object()),
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
    self
  }

  /// Check all the need build widgets and update the widget tree to what need
  /// build widgets want it to be.
  pub fn repair(&mut self, render_tree: &mut RenderTree) {
    while let Some(need_build) = self.pop_need_build_widget() {
      debug_assert!(
        need_build
          .get(self)
          .expect("Must exist!")
          .classify()
          .is_combination(),
        "rebuild widget must be combination widget."
      );

      let mut stack = vec![need_build];

      while let Some(need_build) = stack.pop() {
        let children = need_build
          .get_mut(self)
          .expect("Must exist!")
          .take_children();

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
  }

  /// Tell the render object its owner changed one by one.
  fn flush_to_render(&mut self, render_tree: &mut RenderTree) {
    self.changed_widgets.iter().for_each(|wid| {
      let widget = wid.get(self).expect("Widget should exists!");

      let render_id = *self
        .widget_to_render
        .get(wid)
        .expect("Changed widget should always render widget!");

      let safety = widget.as_render().expect("Must be a render widget!");

      render_id
        .get_mut(render_tree)
        .expect("render object must exists!")
        .update(safety);
    });

    self.changed_widgets.clear();
  }

  /// Try to use `new_widget` to replace widget in old_node and push the
  /// `old_node` into stack, if they have same key. Other, drop the subtree.
  fn try_replace_widget_or_rebuild(
    &mut self,
    node: WidgetId,
    widget: Box<dyn Widget>,
    stack: &mut Vec<WidgetId>,
    render_tree: &mut RenderTree,
  ) {
    let same_key = widget
      .key()
      .and_then(|key| node.get(self).map(|w| Some(key) == w.key()))
      .unwrap_or(false);
    if same_key {
      node.replace(self, widget.into());
      node.mark_changed(self);
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
    new_children: Vec<Box<dyn Widget>>,
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
      .map(|id| id.ancestors(self).find(|id| self.need_builds.contains(id)))
      .flatten();
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
    if self.get(tree).unwrap().classify().is_render() {
      tree.changed_widgets.insert(self);
    } else {
      // Combination widget has no render object to update!"
    }
  }

  /// mark this widget need to build in the next frame.
  pub fn mark_needs_build(self, tree: &mut WidgetTree) {
    debug_assert!(self.get(&tree).unwrap().classify().is_combination());
    tree.need_builds.insert(self);
  }

  /// Returns a reference to the node data.
  pub fn get(self, tree: &WidgetTree) -> Option<WidgetRef> {
    tree.arena.get(self.0).map(|node| node.get().borrow())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut(self, tree: &mut WidgetTree) -> Option<WidgetRefMut> {
    tree
      .arena
      .get_mut(self.0)
      .map(|node| node.get_mut().borrow_mut())
  }

  /// Replace the widget back the widget id, return true if replace successful
  /// and false if the widget id is not valid.
  fn replace(self, tree: &mut WidgetTree, widget: WidgetNode) -> bool {
    if let Some(node) = tree.arena.get_mut(self.0) {
      *node.get_mut() = widget;
      true
    } else {
      false
    }
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
  fn relative_to_render(self, tree: &mut WidgetTree) -> Option<RenderId> {
    let wid = self.down_nearest_render_widget(tree);
    tree.widget_to_render.get(&wid).cloned()
  }

  fn append_widget(self, data: Box<dyn Widget>, tree: &mut WidgetTree) -> WidgetId {
    let child = tree.new_node(data);
    self.0.append(child.0, &mut tree.arena);
    child
  }

  /// A proxy for [NodeId::remove](indextree::NodeId.remove)
  #[allow(dead_code)]
  fn remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena); }

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
        .map_or(false, |node| node.get().borrow().classify().is_render())
      {
        widget_to_render.remove(&wid);
      }
      changed_widgets.remove(&wid);
      need_builds.remove(&wid);
    });

    rid.drop(render_tree);
    // Todo: should remove in a more directly way and not care about
    // relationship
    // Fixme: memory leak here, node just detach and not remove. Wait a pr to
    // provide a method to drop a subtree in indextree.
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
    while wid
      .get(tree)
      .map_or(false, |w| w.classify().is_combination())
    {
      wid = wid.single_child(tree);
    }
    debug_assert!(wid.get(tree).map_or(false, |w| w.classify().is_render()));
    wid
  }

  fn node_feature<F: Fn(&Node<WidgetNode>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }
}

impl dyn Widget {
  fn key(&self) -> Option<&Key> { self.downcast_ref::<KeyDetect>().map(|k| k.key()) }
  fn take_children(&mut self) -> Option<Vec<Box<dyn Widget>>> {
    match self.classify_mut() {
      WidgetClassifyMut::Combination(c) => Some(vec![c.build()]),
      WidgetClassifyMut::SingleChild(r) => Some(vec![r.take_child()]),
      WidgetClassifyMut::MultiChild(multi) => Some(multi.take_children()),
      WidgetClassifyMut::Render(_) => None,
    }
  }

  fn as_render(&self) -> Option<&dyn RenderWidgetSafety> {
    match self.classify() {
      WidgetClassify::Combination(_) => None,
      WidgetClassify::SingleChild(s) => Some(s.as_render()),
      WidgetClassify::MultiChild(m) => Some(m.as_render()),
      WidgetClassify::Render(r) => Some(r.as_render()),
    }
  }
}

pub enum WidgetNode {
  Rc(Rc<RefCell<Box<dyn Widget>>>),
  Widget(Box<dyn Widget>),
}
pub enum WidgetRef<'a> {
  BorrowRef(Ref<'a, dyn Widget>),
  Ref(&'a dyn Widget),
}

pub enum WidgetRefMut<'a> {
  BorrowRefMut(RefMut<'a, dyn Widget>),
  RefMut(&'a mut dyn Widget),
}

impl std::fmt::Debug for WidgetNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.borrow().fmt(f) }
}
impl WidgetNode {
  fn borrow_mut(&mut self) -> WidgetRefMut {
    match self {
      WidgetNode::Rc(rc) => WidgetRefMut::BorrowRefMut(RefMut::map(rc.borrow_mut(), |w| &mut **w)),
      WidgetNode::Widget(w) => WidgetRefMut::RefMut(&mut **w),
    }
  }

  fn borrow(&self) -> WidgetRef {
    match self {
      WidgetNode::Rc(rc) => WidgetRef::BorrowRef(Ref::map(rc.borrow(), |w| &**w)),
      WidgetNode::Widget(w) => WidgetRef::Ref(&**w),
    }
  }
}

use std::ops::{Deref, DerefMut};
impl<'a> Deref for WidgetRef<'a> {
  type Target = dyn Widget;
  fn deref(&self) -> &Self::Target {
    match self {
      WidgetRef::Ref(r) => *r,
      WidgetRef::BorrowRef(r) => &**r,
    }
  }
}

impl<'a> Deref for WidgetRefMut<'a> {
  type Target = dyn Widget;
  fn deref(&self) -> &Self::Target {
    match self {
      WidgetRefMut::RefMut(r) => *r,
      WidgetRefMut::BorrowRefMut(r) => &**r,
    }
  }
}

impl<'a> DerefMut for WidgetRefMut<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      WidgetRefMut::RefMut(r) => *r,
      WidgetRefMut::BorrowRefMut(r) => &mut **r,
    }
  }
}

impl From<Box<dyn Widget>> for WidgetNode {
  #[inline]
  fn from(w: Box<dyn Widget>) -> Self { WidgetNode::Widget(w) }
}

impl From<Rc<RefCell<Box<dyn Widget>>>> for WidgetNode {
  #[inline]
  fn from(w: Rc<RefCell<Box<dyn Widget>>>) -> Self { WidgetNode::Rc(w) }
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
    tree.set_root(EmbedPost::new(level).into(), &mut render_tree);
    tree
  }

  #[test]
  fn inflate_tree() {
    let (widget_tree, render_tree) = create_embed_app(3);

    assert_eq!(
      widget_tree.symbol_shape(),
      r#"EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 3 }
└── RowColumn { axis: Horizontal, children: [] }
    ├── Text("Simple demo")
    ├── Text("Adoo")
    ├── Text("Recursive x times")
    └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 2 }
        └── RowColumn { axis: Horizontal, children: [] }
            ├── Text("Simple demo")
            ├── Text("Adoo")
            ├── Text("Recursive x times")
            └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 1 }
                └── RowColumn { axis: Horizontal, children: [] }
                    ├── Text("Simple demo")
                    ├── Text("Adoo")
                    ├── Text("Recursive x times")
                    └── EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 0 }
                        └── RowColumn { axis: Horizontal, children: [] }
                            ├── Text("Simple demo")
                            ├── Text("Adoo")
                            └── Text("Recursive x times")
"#
    );

    assert_eq!(
      render_tree.symbol_shape(),
      r#"RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
├── TextRender { text: "Simple demo" }
├── TextRender { text: "Adoo" }
├── TextRender { text: "Recursive x times" }
└── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
    ├── TextRender { text: "Simple demo" }
    ├── TextRender { text: "Adoo" }
    ├── TextRender { text: "Recursive x times" }
    └── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
        ├── TextRender { text: "Simple demo" }
        ├── TextRender { text: "Adoo" }
        ├── TextRender { text: "Recursive x times" }
        └── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
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
└── KeyDetect { key: KI4(0), child: RowColumn { axis: Horizontal, children: [] } }
    ├── KeyDetect { key: KI4(0), child: Text("New title") }
    ├── KeyDetect { key: KI4(1), child: Text("") }
    ├── KeyDetect { key: KI4(2), child: Text("") }
    └── KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 2 } }
        └── KeyDetect { key: KI4(0), child: RowColumn { axis: Horizontal, children: [] } }
            ├── KeyDetect { key: KI4(0), child: Text("New title") }
            ├── KeyDetect { key: KI4(1), child: Text("") }
            ├── KeyDetect { key: KI4(2), child: Text("") }
            └── KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 1 } }
                └── KeyDetect { key: KI4(0), child: RowColumn { axis: Horizontal, children: [] } }
                    ├── KeyDetect { key: KI4(0), child: Text("New title") }
                    ├── KeyDetect { key: KI4(1), child: Text("") }
                    ├── KeyDetect { key: KI4(2), child: Text("") }
                    └── KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 0 } }
                        └── KeyDetect { key: KI4(0), child: RowColumn { axis: Horizontal, children: [] } }
                            ├── KeyDetect { key: KI4(0), child: Text("New title") }
                            ├── KeyDetect { key: KI4(1), child: Text("") }
                            └── KeyDetect { key: KI4(2), child: Text("") }
"#
    );

    assert_eq!(
      env.render_tree.symbol_shape(),
      r#"RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
├── TextRender { text: "New title" }
├── TextRender { text: "" }
├── TextRender { text: "" }
└── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
    ├── TextRender { text: "New title" }
    ├── TextRender { text: "" }
    ├── TextRender { text: "" }
    └── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
        ├── TextRender { text: "New title" }
        ├── TextRender { text: "" }
        ├── TextRender { text: "" }
        └── RowColRender { flex: FlexContainer { axis: Horizontal, bound: BoxLayout { constraints: EFFECTED_BY_CHILDREN, box_bound: None } } }
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
    widget_tree.set_root(root.into(), &mut render_tree);
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
