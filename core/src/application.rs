use crate::{
  render_object::*, util::TreeFormatter, widget::key::Key, widget::*,
};
use ::herald::prelude::*;
use indextree::*;
use smallvec::{smallvec, SmallVec};
use std::{
  collections::{HashMap, HashSet},
};
mod tree_relationship;
use tree_relationship::Relationship;

#[derive(Debug)]
enum WidgetNode {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
}

impl WidgetNode {
  fn key(&self) -> Option<&Key> {
    match self {
      Self::Combination(c) => c.key(),
      Self::Render(r) => r.key(),
    }
  }
}

#[derive(Default)]
pub struct Application<'a> {
  notifier: LocalSubject<'a, (), ()>,
  w_arena: Arena<WidgetNode>,
  r_arena: Arena<Box<dyn RenderObject>>,
  widget_tree: Option<NodeId>,
  render_tree: Option<NodeId>,
  tree_relationship: Relationship,
  /// Store widgets that modified and wait to update its corresponds render
  /// objects in render tree.
  dirty_widgets: HashSet<NodeId>,
  /// Store combination widgets that has require to rebuild its subtree.
  wait_rebuilds: HashSet<NodeId>,
}

impl<'a> Application<'a> {
  #[inline]
  pub fn new() -> Application<'a> { Default::default() }

  pub fn run(mut self, w: Widget) {
    self.inflate(w);
    self.construct_render_tree(
      self.widget_tree.expect("widget root should exists"),
    );

    todo!(
      "
      1. update widget tree & render tree when change occurs;
      2. start a event loop to handle event.
      3. run layout and paint for it.
    "
    );

    self.repair_tree();
  }

  /// inflate widget tree, so every widget tree leaf should be a render object.
  fn inflate(&mut self, w: Widget) {
    let (widget_node, children) = Self::consume_widget_to_node(w);
    let root = self.w_arena.new_node(widget_node);
    self.widget_tree = Some(root);

    if let Some(c) = children {
      self.inflate_widget_subtree(root, c);
    }
  }

  /// Return an widget after inflated, and its children
  #[inline]
  fn consume_widget_to_node(
    widget: Widget,
  ) -> (WidgetNode, Option<SmallVec<[Widget; 1]>>) {
    match widget {
      Widget::Combination(w) => {
        let c = w.build();
        (WidgetNode::Combination(w), Some(smallvec![c]))
      }
      Widget::Render(r) => (WidgetNode::Render(r), None),
      Widget::SingleChild(w) => {
        let (render, child) = w.split();
        (WidgetNode::Render(render), Some(smallvec![child]))
      }
      Widget::MultiChild(w) => {
        let (render, children) = w.split();
        (
          WidgetNode::Render(render),
          Some(SmallVec::from(children)),
        )
      }
    }
  }

  fn inflate_widget_subtree(
    &mut self,
    sub_tree: NodeId,
    children: SmallVec<[Widget; 1]>,
  ) {
    let mut stack = vec![(sub_tree, children)];

    while let Some((parent, mut children)) = stack.pop() {
      while let Some(child) = children.pop() {
        let (node, c_children) = Self::consume_widget_to_node(child);
        let new_id = self.preappend_widget(parent, node);
        if let Some(c_children) = c_children {
          stack.push((parent, children));
          stack.push((new_id, c_children));
          break;
        }
      }
    }
  }

  fn down_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let WidgetNode::Combination(_) = self.w_arena[wid].get() {
      // combination widget always have single child.
      debug_assert_eq!(wid.children(&self.w_arena).count(), 1);

      wid = self.w_arena[wid]
        .first_child()
        .expect("Combination node must be only one child")
    }
    debug_assert!(matches!(
      &self.w_arena[wid].get(),
      WidgetNode::Render(_)
    ));

    wid
  }

  fn upper_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let WidgetNode::Combination(_) = self.w_arena[wid].get() {
      wid = self.w_arena[wid].parent().expect(
        "should only call this method if `wid`  have render widget ancestor!",
      );
    }
    debug_assert!(matches!(
      &self.w_arena[wid].get(),
      WidgetNode::Render(_)
    ));

    wid
  }

  /// construct a render tree correspond to widget tree `wid`.
  fn construct_render_tree(&mut self, wid: NodeId) {
    let mut r_wid = self.down_to_render_widget(wid);
    let rid;
    if self.render_tree.is_none() {
      rid = self.r_arena.new_node(self.create_render_object(r_wid));
      self.render_tree = Some(rid);
      self.tree_relationship.bind(r_wid, rid);
    } else if let Some(render_id) =
      self.tree_relationship.widget_to_render(r_wid)
    {
      rid = *render_id;
    } else {
      let rw_parent = self.upper_to_render_widget(
        self.w_arena[wid]
          .parent()
          .expect("should not be a root widget"),
      );
      let p_rid = *self.tree_relationship.widget_to_render(rw_parent).expect(
        "parent render object node should construct before construct subtree",
      );
      let (render_widget, render_object) =
        self.append_render_node(r_wid, p_rid);
      r_wid = render_widget;
      rid = render_object;
    }

    let mut stack = vec![];
    self.render_tree_depth_construct(r_wid, rid, &mut stack);
    while let Some((wid, rid)) = stack.pop() {
      if let Some(sibling) = self.w_arena[wid].next_sibling() {
        let (render_widget, render_object) =
          self.append_render_node(sibling, rid);
        stack.push((sibling, rid));
        self.render_tree_depth_construct(
          render_widget,
          render_object,
          &mut stack,
        );
      }
    }
  }

  fn render_tree_depth_construct(
    &mut self,
    mut wid: NodeId,
    mut rid: NodeId,
    stack: &mut Vec<(NodeId, NodeId)>,
  ) {
    wid = self.down_to_render_widget(wid);

    while let Some(w_child_id) = self.w_arena[wid].first_child() {
      let (w_child_id, render_object_id) =
        self.append_render_node(w_child_id, rid);
      stack.push((w_child_id, rid));
      rid = render_object_id;
      wid = w_child_id;
    }
  }

  /// Use `wid` to create a render object, and append it into rid.
  /// Return the render widget id which created the render object and the
  /// created render object id.
  fn append_render_node(
    &mut self,
    mut wid: NodeId,
    rid: NodeId,
  ) -> (NodeId, NodeId) {
    wid = self.down_to_render_widget(wid);
    let r_child = self.r_arena.new_node(self.create_render_object(wid));
    rid.append(r_child, &mut self.r_arena);
    self.tree_relationship.bind(wid, r_child);
    (wid, r_child)
  }

  fn create_render_object(&self, render_wid: NodeId) -> Box<dyn RenderObject> {
    let render_object = if let WidgetNode::Render(ref r) =
      self.w_arena[render_wid].get()
    {
      r.create_render_object()
    } else {
      unreachable!("only render widget can create render object!");
    };
    render_object
  }

  fn repair_tree(&mut self) {
    let mut repair_stack = vec![];
    while let Some(first) = self.wait_rebuilds.iter().nth(0).map(|id| *id) {
      // Always find the topmost widget which need to rebuild to rebuild
      // subtree.
      if let Some(top) = self.get_rebuild_ancestors(first) {
        if let Some(sub_root) = self.w_arena.get_mut(top) {
          debug_assert!(
            matches!(sub_root.get(), WidgetNode::Combination(_)),
            "rebuild widget must be combination widget."
          );

          // combination widget should have only one child."
          debug_assert!(sub_root.first_child().is_some());
          debug_assert_eq!(sub_root.first_child(), sub_root.last_child());

          if let WidgetNode::Combination(ref c) = sub_root.get() {
            let new_widget = c.build();
            let old_node =
              sub_root.first_child().expect("should have single child");

            repair_stack.push((old_node, new_widget));
            while let Some((node, widget)) = repair_stack.pop() {
              self.repair_subtree(node, widget, &mut repair_stack);
            }
            self.wait_rebuilds.remove(&top);
          }
        }
      } else {
        self.wait_rebuilds.remove(&first);
      }
    }
  }

  /// Keep the `widget_subtree` to correct newest state, across minimal
  /// reconstruct or replace node in the subtree.
  fn repair_subtree(
    &mut self,
    old_node_id: NodeId,
    new_widget: Widget,
    stack: &mut Vec<(NodeId, Widget)>,
  ) {
    let old_key = self.w_arena[old_node_id].get().key();
    if old_key.is_some() && old_key == new_widget.key() {
      self.wait_rebuilds.remove(&old_node_id);
      // keep node, but replace widget in node with new widget.
      let (w, children) = Self::consume_widget_to_node(new_widget);
      *self.w_arena[old_node_id].get_mut() = w;
      self.dirty_widgets.insert(old_node_id);

      if let Some(widgets) = children {
        self.repair_children_by_key(old_node_id, widgets, stack);
      } else {
        // There is no children in new widget, drop all old children.
        let mut child = self.w_arena[old_node_id].first_child();
        while let Some(c) = child {
          child = self.w_arena[c].next_sibling();
          self.drop_subtree(c);
        }
      }
    } else {
      self.rebuild_subtree(old_node_id, new_widget);
    }

    self.wait_rebuilds.remove(&old_node_id);
  }

  /// rebuild the subtree `wid` by the new children `new_children`, the same key
  /// children as before will keep the old subtree and will add into the `stack`
  /// to recursive repair, else will construct a new subtree.
  fn repair_children_by_key(
    &mut self,
    wid: NodeId,
    new_children: SmallVec<[Widget; 1]>,
    stack: &mut Vec<(NodeId, Widget)>,
  ) {
    let mut key_children = HashMap::new();
    let mut child = self.w_arena[wid].first_child();
    while let Some(id) = child {
      child = self.w_arena[id].next_sibling();
      let key = self.w_arena[id].get().key().map(|k| k.clone());
      if let Some(key) = key {
        id.detach(&mut self.w_arena);
        key_children.insert(key, id);
      } else {
        self.drop_subtree(id);
      }
    }

    for w in new_children.into_iter() {
      if let Some(k) = w.key() {
        if let Some(id) = key_children.get(k).map(|id| *id) {
          key_children.remove(k);
          wid.append(id, &mut self.w_arena);
          stack.push((id, w));
          continue;
        }
      }

      let (w, children) = Self::consume_widget_to_node(w);
      let child_id = self.append_widget(wid, w);
      if let Some(children) = children {
        self.inflate_widget_subtree(child_id, children);
        self.construct_render_tree(child_id);
      }
    }

    key_children.into_iter().for_each(|(_, v)| {
      self.drop_subtree(v);
    });
  }

  fn rebuild_subtree(&mut self, old_node: NodeId, new_widget: Widget) {
    let parent_id = self.w_arena[old_node]
      .parent()
      .expect("parent should exists!");
    self.drop_subtree(old_node);

    let (w, children) = Self::consume_widget_to_node(new_widget);
    let new_child_id = self.append_widget(parent_id, w);
    if let Some(children) = children {
      self.inflate_widget_subtree(new_child_id, children);
    }
    self.construct_render_tree(new_child_id);
  }

  fn drop_subtree(&mut self, wid: NodeId) {
    let rid = *self
      .tree_relationship
      .widget_to_render(self.down_to_render_widget(wid))
      .expect("must exist");

    let Self {
      w_arena,
      tree_relationship,
      dirty_widgets,
      wait_rebuilds,
      ..
    } = self;

    wid.descendants(w_arena).for_each(|id| {
      // clear relationship between render object and render widget.
      if matches!(w_arena[id].get(), WidgetNode::Render(_)) {
        tree_relationship.unbind(id)
      }
      dirty_widgets.remove(&id);
      wait_rebuilds.remove(&id);
    });

    // Todo: should remove in a more directly way and not care about
    // relationship
    // Fixme: memory leak here, node not remove.
    wid.detach(&mut self.w_arena);
    rid.detach(&mut self.r_arena);
    if self.widget_tree == Some(wid) {
      self.widget_tree = None;
    }
    if self.render_tree == Some(rid) {
      self.render_tree = None;
    }
  }

  fn get_rebuild_ancestors(&self, wid: NodeId) -> Option<NodeId> {
    wid
      .ancestors(&self.w_arena)
      .filter(|id| self.wait_rebuilds.contains(id))
      .last()
      .or(Some(wid))
  }

  fn append_widget(&mut self, wid: NodeId, w: WidgetNode) -> NodeId {
    let child = self.w_arena.new_node(w);
    wid.append(child, &mut self.w_arena);
    child
  }

  fn preappend_widget(&mut self, wid: NodeId, w: WidgetNode) -> NodeId {
    let child = self.w_arena.new_node(w);
    wid.prepend(child, &mut self.w_arena);
    child
  }


  #[allow(dead_code)]
  pub(crate) fn widget_symbol_tree(&self) -> String {
    if let Some(w_root) = self.widget_tree {
      format!("{:?}", TreeFormatter::new(&self.w_arena, w_root))
    } else {
      "".to_owned()
    }
  }

  #[allow(dead_code)]
  pub(crate) fn render_symbol_tree(&self) -> String {
    if let Some(r_root) = self.render_tree {
      format!("{:?}", TreeFormatter::new(&self.r_arena, r_root))
    } else {
      "".to_owned()
    }
  }
}

#[cfg(test)]
mod test {
  use std::fmt::{Debug, Formatter, Result};
  use super::*;
  use crate::widget::Row;
  use crate::{render_ctx::*, render_object_box::*};
  extern crate test;
  use test::Bencher;

  #[derive(Clone, Debug)]
  struct EmbedPost {
    title: &'static str,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl<'a> CombinationWidget<'a> for EmbedPost {
    fn build(&self) -> Widget {
      let mut row = Row {
        children: vec![
          Text(self.title).to_widget(),
          Text(self.author).to_widget(),
          Text(self.content).to_widget(),
        ],
      };
      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        row.children.push(embed.to_widget())
      }
      row.to_widget()
    }
  }

  fn create_embed_app<'a>(level: usize) -> Application<'a> {
    let post = EmbedPost {
      title: "Simple demo",
      author: "Adoo",
      content: "Recursive x times",
      level,
    };

    let mut app = Application::new();
    app.inflate(post.to_widget());
    app.construct_render_tree(app.widget_tree.expect("must exists"));
    app
  }

  #[test]
  fn widget_tree_inflate() {
    let app = create_embed_app(3);

    assert_eq!(
      app.widget_symbol_tree(),
      r#"Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 3 })
└── Render(RenderRow)
    ├── Render(Text("Simple demo"))
    ├── Render(Text("Adoo"))
    ├── Render(Text("Recursive x times"))
    └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 2 })
        └── Render(RenderRow)
            ├── Render(Text("Simple demo"))
            ├── Render(Text("Adoo"))
            ├── Render(Text("Recursive x times"))
            └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 1 })
                └── Render(RenderRow)
                    ├── Render(Text("Simple demo"))
                    ├── Render(Text("Adoo"))
                    ├── Render(Text("Recursive x times"))
                    └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 0 })
                        └── Render(RenderRow)
                            ├── Render(Text("Simple demo"))
                            ├── Render(Text("Adoo"))
                            └── Render(Text("Recursive x times"))
"#
    );

    assert_eq!(
      app.render_symbol_tree(),
      r#"RowRenderObject { inner_layout: [], size: None }
├── Text("Simple demo")
├── Text("Adoo")
├── Text("Recursive x times")
└── RowRenderObject { inner_layout: [], size: None }
    ├── Text("Simple demo")
    ├── Text("Adoo")
    ├── Text("Recursive x times")
    └── RowRenderObject { inner_layout: [], size: None }
        ├── Text("Simple demo")
        ├── Text("Adoo")
        ├── Text("Recursive x times")
        └── RowRenderObject { inner_layout: [], size: None }
            ├── Text("Simple demo")
            ├── Text("Adoo")
            └── Text("Recursive x times")
"#
    );
  }

  #[test]
  fn drop_subtree() {
    let mut app = create_embed_app(3);
    let id = app.widget_tree.unwrap();
    app.drop_subtree(id);

    assert!(app.tree_relationship.is_empty());
    assert!(app.dirty_widgets.is_empty());
    assert!(app.wait_rebuilds.is_empty());

    assert!(app.widget_tree.is_none());
    assert!(app.render_tree.is_none());
  }

  use std::{cell::RefCell, rc::Rc};
  #[derive(Clone, Default)]
  struct EmbedKeyPost {
    title: Rc<RefCell<&'static str>>,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl Debug for EmbedKeyPost {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
      f.debug_struct("EmbedKeyPost")
        .field("title", &self.title)
        .field("author", &self.author)
        .field("content", &self.content)
        .field("level", &self.level)
        .finish()
    }
  }

  impl<'a> CombinationWidget<'a> for EmbedKeyPost {
    fn build(&self) -> Widget {
      let mut row = Row {
        children: vec![
          KeyDetect::new(0, Text(*self.title.borrow())).to_widget(),
          KeyDetect::new(1, Text(self.author)).to_widget(),
          KeyDetect::new(2, Text(self.content)).to_widget(),
        ],
      };
      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        row
          .children
          .push(KeyDetect::new("embed", embed).to_widget())
      }
      KeyDetect::new(0, row).to_widget()
    }
  }

  #[derive(Default)]
  struct KeyDetectEnv<'a> {
    app: Application<'a>,
    title: Option<Rc<RefCell<&'static str>>>,
  }

  impl<'a> KeyDetectEnv<'a> {
    fn construct_tree(&mut self, level: usize) -> &mut Self {
      let mut post = EmbedKeyPost::default();
      post.level = level;
      let title = post.title.clone();
      self.title = Some(title);

      self.app.inflate(post.clone().to_widget());
      self
        .app
        .construct_render_tree(self.app.widget_tree.unwrap());

      self
    }

    fn emit_rebuild(&mut self) {
      *self.title.as_mut().unwrap().borrow_mut() = "New title";
      self.app.wait_rebuilds.insert(self.app.widget_tree.unwrap());
    }
  }

  #[test]
  fn repair_tree() {
    let mut env = KeyDetectEnv::default();
    env.construct_tree(3).emit_rebuild();

    // fixme: below assert should failed, after support update render tree data.
    assert_eq!(
      env.app.widget_symbol_tree(),
      r#"Combination(EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 3 })
└── Render(KeyRender { key: KI4(0), render: RenderRow })
    ├── Render(KeyDetect { key: KI4(0), child: Text("") })
    ├── Render(KeyDetect { key: KI4(1), child: Text("") })
    ├── Render(KeyDetect { key: KI4(2), child: Text("") })
    └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 2 } })
        └── Render(KeyRender { key: KI4(0), render: RenderRow })
            ├── Render(KeyDetect { key: KI4(0), child: Text("") })
            ├── Render(KeyDetect { key: KI4(1), child: Text("") })
            ├── Render(KeyDetect { key: KI4(2), child: Text("") })
            └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 1 } })
                └── Render(KeyRender { key: KI4(0), render: RenderRow })
                    ├── Render(KeyDetect { key: KI4(0), child: Text("") })
                    ├── Render(KeyDetect { key: KI4(1), child: Text("") })
                    ├── Render(KeyDetect { key: KI4(2), child: Text("") })
                    └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 0 } })
                        └── Render(KeyRender { key: KI4(0), render: RenderRow })
                            ├── Render(KeyDetect { key: KI4(0), child: Text("") })
                            ├── Render(KeyDetect { key: KI4(1), child: Text("") })
                            └── Render(KeyDetect { key: KI4(2), child: Text("") })
"#
    );

    // fixme: below assert should failed, after support update render tree.
    assert_eq!(
      env.app.render_symbol_tree(),
      r#"RowRenderObject { inner_layout: [], size: None }
├── Text("")
├── Text("")
├── Text("")
└── RowRenderObject { inner_layout: [], size: None }
    ├── Text("")
    ├── Text("")
    ├── Text("")
    └── RowRenderObject { inner_layout: [], size: None }
        ├── Text("")
        ├── Text("")
        ├── Text("")
        └── RowRenderObject { inner_layout: [], size: None }
            ├── Text("")
            ├── Text("")
            └── Text("")
"#
    );
  }

  fn assert_root_bound(app: &mut Application, bound: Option<Size>) {
    let root = app.r_arena.get_mut(app.render_tree.unwrap()).unwrap();
    let render_box = root.get_mut().to_render_box().unwrap();
    assert_eq!(render_box.bound(), bound);
  }

  fn layout_app(app: &mut Application) {
    let mut_ptr = &mut app.r_arena as *mut Arena<Box<dyn RenderObject>>;
    let root = app.r_arena.get_mut(app.render_tree.unwrap()).unwrap();
    unsafe {
      root.get_mut().perform_layout(
        app.render_tree.unwrap(),
        &mut RenderCtx::new(&mut *mut_ptr),
      );
    }
  }

  fn mark_dirty(app: &mut Application, node_id: NodeId) {
    let mut_ptr = &mut app.r_arena as *mut Arena<Box<dyn RenderObject>>;
    unsafe {
      app
        .r_arena
        .get_mut(node_id)
        .unwrap()
        .get_mut()
        .mark_dirty(node_id, &mut RenderCtx::new(&mut *mut_ptr));
    }
  }

  #[bench]
  fn widget_tree_deep_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost {
        title: "Simple demo",
        author: "Adoo",
        content: "Recursive 1000 times",
        level: 1000,
      };
      let mut app = Application::new();
      app.inflate(post.to_widget());
    });
  }

  #[test]
  fn test_layout() {
    let post = EmbedPost {
      title: "Simple demo",
      author: "Adoo",
      content: "Recursive 5 times",
      level: 5,
    };
    let mut app = Application::new();
    app.inflate(post.to_widget());
    app.construct_render_tree(app.widget_tree.unwrap());

    layout_app(&mut app);
    assert_root_bound(
      &mut app,
      Some(Size {
        width: 192,
        height: 1,
      }),
    );

    let last_child_id = app
      .r_arena
      .get(app.render_tree.unwrap())
      .unwrap()
      .last_child()
      .unwrap();
    mark_dirty(&mut app, last_child_id);
    assert_root_bound(&mut app, None);

    layout_app(&mut app);
    assert_root_bound(
      &mut app,
      Some(Size {
        width: 192,
        height: 1,
      }),
    );
  }

  #[bench]
  fn widget_tree_deep_1000_with_key(b: &mut Bencher) {
    let mut env = KeyDetectEnv::default();
    env.construct_tree(1000);
    b.iter(|| {
      env.emit_rebuild();
      env.app.repair_tree();
    });
  }

  #[bench]
  fn render_tree_deep_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost {
        title: "Simple demo",
        author: "Adoo",
        content: "Recursive 1000 times",
        level: 1000,
      };
      let mut app = Application::new();
      app.inflate(post.to_widget());
      app.construct_render_tree(app.widget_tree.expect("must exists"));
    });
  }
}
