use crate::{prelude::*, render::render_tree::*, util::TreeFormatter};
use std::collections::{HashMap, HashSet};

use indextree::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);

#[derive(Default)]
pub struct WidgetTree {
  arena: Arena<Box<dyn Widget>>,
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

  pub(crate) fn set_root(
    &mut self,
    data: Box<dyn Widget>,
    render_tree: &mut RenderTree,
  ) -> WidgetId {
    debug_assert!(self.root.is_none());
    let root = self.new_node(data);
    self.root = Some(root);
    self.inflate(root, render_tree);
    root
  }

  #[inline]
  pub(crate) fn new_node(&mut self, data: Box<dyn Widget>) -> WidgetId {
    WidgetId(self.arena.new_node(data))
  }

  /// inflate  subtree, so every subtree leaf should be a Widget::Render.
  pub(crate) fn inflate(&mut self, wid: WidgetId, render_tree: &mut RenderTree) -> &mut Self {
    let parent_id = wid
      .ancestors(self)
      .find(|id| !matches!(id.classify(self), Some(WidgetClassify::Combination(_))))
      .map(|id| self.widget_to_render.get(&id))
      .flatten()
      .copied();
    let mut stack = vec![(wid, parent_id)];

    macro make_pair($widget: ident, $wid: ident, $parent_rid: ident) {{
      let render_obj = $widget.create_render_object();
      let rid = if let Some(id) = $parent_rid {
        id.prepend_object($wid, render_obj, render_tree)
      } else {
        render_tree.set_root($wid, render_obj)
      };
      self.widget_to_render.insert($wid, rid);
      rid
    }}

    while let Some((wid, parent_rid)) = stack.pop() {
      let p_widget = wid.classify_mut(self).expect("must exist!");
      match p_widget {
        WidgetClassifyMut::Combination(c) => {
          let child = wid.append_widget(c.build(), self);
          stack.push((child, parent_rid));
        }
        WidgetClassifyMut::SingleChild(single) => {
          let child = single.take_child();
          let rid = make_pair!(single, wid, parent_rid);
          let child = wid.append_widget(child, self);
          stack.push((child, Some(rid)));
        }
        WidgetClassifyMut::MultiChild(multi) => {
          let children = multi.take_children();
          let rid = make_pair!(multi, wid, parent_rid);
          children.into_iter().for_each(|w| {
            let id = wid.append_widget(w, self);
            stack.push((id, Some(rid)));
          });
        }
        WidgetClassifyMut::Render(render) => {
          make_pair!(render, wid, parent_rid);
        }
      }
    }
    self
  }

  /// Check all the need build widgets and update the widget tree to what need
  /// build widgets want it to be.
  pub(crate) fn repair(&mut self, render_tree: &mut RenderTree) {
    while let Some(need_build) = self.pop_need_build_widget() {
      debug_assert!(
        matches!(
          need_build.classify(self).expect("Must exist!"),
          WidgetClassify::Combination(_)
        ),
        "rebuild widget must be combination widget."
      );

      let mut stack = vec![need_build];

      while let Some(need_build) = stack.pop() {
        let widget = need_build.classify_mut(self).expect("Must exist!");
        match widget {
          WidgetClassifyMut::Combination(c) => {
            let new_child = c.build();
            let old_child_node = need_build.single_child(&self);
            self.try_replace_widget_or_rebuild(old_child_node, new_child, &mut stack, render_tree);
          }
          WidgetClassifyMut::SingleChild(r) => {
            let new_child = r.take_child();
            let old_child_node = need_build.single_child(&self);
            self.try_replace_widget_or_rebuild(old_child_node, new_child, &mut stack, render_tree);
          }
          WidgetClassifyMut::MultiChild(multi) => {
            let new_children = multi.take_children();
            self.repair_children_by_key(need_build, new_children, &mut stack, render_tree);
          }
          WidgetClassifyMut::Render(_) => {
            // down to leaf, nothing to do.
          }
        }
      }
    }

    self.flush_to_render(render_tree);
  }

  /// Tell the render object its owner changed one by one.
  fn flush_to_render(&mut self, render_tree: &mut RenderTree) {
    self.changed_widgets.iter().for_each(|wid| {
      let widget = wid.classify(self).expect("Widget should exists!");
      let render_id = *self
        .widget_to_render
        .get(wid)
        .expect("Changed widget should always render widget!");

      let safety = widget.try_as_render().expect("Must be a render widget!");
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
    let key = widget.key();
    if key.is_some() && key == node.key(self) {
      node.replace(self, widget);
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
      let key = id.key(self);
      if let Some(key) = key {
        let key = key.clone();
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
          node.append(id, self);
          self.try_replace_widget_or_rebuild(id, w, stack, render_tree);
          continue;
        }
      }

      let child_id = node.append_widget(w, self);
      self.inflate(child_id, render_tree);
    }

    key_children.into_iter().for_each(|(_, v)| {
      v.drop(self, render_tree);
    });
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
    if !matches!(self.classify(tree).unwrap(), WidgetClassify::Combination(_)) {
      tree.changed_widgets.insert(self);
    } else {
      // Combination widget has no render object to update!"
    }
  }

  /// mark this widget need to build in the next frame.
  pub fn mark_needs_build(self, tree: &mut WidgetTree) {
    debug_assert!(matches!(
      self.classify(tree).unwrap(),
      WidgetClassify::Combination(_)
    ));
    tree.need_builds.insert(self);
  }

  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &WidgetTree) -> Option<&dyn Widget> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut (dyn Widget + 'static)> {
    tree.arena.get_mut(self.0).map(|node| &mut **node.get_mut())
  }

  /// Replace the widget back the widget id, return true if replace successful
  /// and false if the widget id is not valid.
  pub(crate) fn replace(self, tree: &mut WidgetTree, widget: Box<dyn Widget>) -> bool {
    if let Some(node) = tree.arena.get_mut(self.0) {
      *node.get_mut() = widget;
      true
    } else {
      false
    }
  }

  /// classify the widget back in this id, and return its reference
  pub(crate) fn classify(self, tree: &WidgetTree) -> Option<WidgetClassify> {
    self.get(tree).map(|w| w.classify())
  }

  /// classify the widget back in this id, and return its mutation reference.
  pub(crate) fn classify_mut(self, tree: &mut WidgetTree) -> Option<WidgetClassifyMut> {
    self.get_mut(tree).map(|w| w.classify_mut())
  }

  /// A delegate for [NodeId::parent](indextree::NodeId.parent)
  pub fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  /// A delegate for [NodeId::first_child](indextree::NodeId.first_child)
  pub fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  /// A delegate for [NodeId::last_child](indextree::NodeId.last_child)
  pub fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub fn previous_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub fn ancestors<'a>(self, tree: &'a WidgetTree) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  pub fn descendants<'a>(self, tree: &'a WidgetTree) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  /// A delegate for [NodeId::detach](indextree::NodeId.detach)
  pub fn detach(self, tree: &mut WidgetTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  /// return the relative render widget.
  pub fn relative_to_render(self, tree: &mut WidgetTree) -> Option<RenderId> {
    let wid = self.down_nearest_render_widget(tree);
    tree.widget_to_render.get(&wid).cloned()
  }

  pub(crate) fn append_widget(self, data: Box<dyn Widget>, tree: &mut WidgetTree) -> WidgetId {
    let child = tree.new_node(data);
    self.append(child, tree);
    child
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  pub(crate) fn append(self, new_child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  #[allow(dead_code)]
  pub(crate) fn remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena); }

  /// Drop the subtree
  pub(crate) fn drop(self, tree: &mut WidgetTree, render_tree: &mut RenderTree) {
    let rid = self.relative_to_render(tree).expect("must exists");
    let WidgetTree {
      widget_to_render,
      arena,
      changed_widgets,
      need_builds,
      ..
    } = tree;
    self.0.descendants(arena).for_each(|id| {
      let wid = WidgetId(id);
      if !matches!(
        arena.get(id).map(|node| node.get().classify()),
        Some(WidgetClassify::Combination(_))
      ) {
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
  pub(crate) fn single_child(self, tree: &WidgetTree) -> WidgetId {
    debug_assert!(self.first_child(tree).is_some());
    debug_assert_eq!(self.first_child(tree), self.last_child(tree));
    self
      .first_child(tree)
      .expect("Caller assert `wid` has single child")
  }

  /// find the nearest render widget in subtree, include self.
  pub(crate) fn down_nearest_render_widget(self, tree: &WidgetTree) -> WidgetId {
    let mut wid = self;
    while let Some(WidgetClassify::Combination(_)) = wid.classify(tree) {
      wid = wid.single_child(tree);
    }
    debug_assert!(!matches!(
      &wid.classify(&tree).unwrap(),
      WidgetClassify::Combination(_)
    ));
    wid
  }

  /// return the widget key of in this node.
  fn key(self, tree: &WidgetTree) -> Option<&Key> {
    self
      .get(tree)
      .map(|w| w.as_any().downcast_ref::<KeyDetect>())
      .flatten()
      .map(|k| k.key())
  }

  fn node_feature<F: Fn(&Node<Box<dyn Widget + '_>>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }
}

impl dyn Widget {
  fn key(&self) -> Option<&Key> { self.as_any().downcast_ref::<KeyDetect>().map(|k| k.key()) }
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
