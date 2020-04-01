use crate::{
  render_object::*, util::TreeFormatter, widget::key::Key, widget::*,
};
use ::herald::prelude::*;
use indextree::*;
use smallvec::{smallvec, SmallVec};
use std::{
  collections::{HashMap, HashSet},
  ptr::NonNull,
};
mod tree_relationship;
use std::any::Any;
use tree_relationship::Relationship;

#[derive(Debug)]
enum WidgetInstance {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
}

impl<'a> WidgetStates<'a> for WidgetInstance {
  #[inline]
  fn changed_emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    match self {
      Self::Combination(c) => c.changed_emitter(notifier),
      Self::Render(r) => r.changed_emitter(notifier),
    }
  }

  #[inline]
  fn as_any(&self) -> Option<&dyn Any> {
    match self {
      Self::Combination(c) => c.as_any(),
      Self::Render(r) => r.as_any(),
    }
  }
}

pub(crate) struct WidgetNode {
  widget: WidgetInstance,
  subscription_guards: (
    Option<SubscriptionGuard<Box<dyn SubscriptionLike>>>,
    Option<SubscriptionGuard<Box<dyn SubscriptionLike>>>,
  ),
}

impl WidgetNode {
  #[inline]
  fn new(w: WidgetInstance) -> Self {
    WidgetNode {
      widget: w,
      subscription_guards: (None, None),
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

  pub fn run<W: Into<Widget>>(mut self, w: W) {
    self.inflate(w.into());
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
    let root = self.w_arena.new_node(WidgetNode::new(widget_node));
    self.widget_tree = Some(root);
    self.track_widget(root);

    if let Some(c) = children {
      self.inflate_widget_subtree(root, c);
    }
  }

  /// Return an widget after inflated, and its children
  #[inline]
  fn consume_widget_to_node(
    widget: Widget,
  ) -> (WidgetInstance, Option<SmallVec<[Widget; 1]>>) {
    match widget {
      Widget::Combination(w) => {
        let c = w.build();
        (WidgetInstance::Combination(w), Some(smallvec![c]))
      }
      Widget::Render(r) => (WidgetInstance::Render(r), None),
      Widget::SingleChild(w) => {
        let (render, child) = w.split();
        (WidgetInstance::Render(render), Some(smallvec![child]))
      }
      Widget::MultiChild(w) => {
        let (render, children) = w.split();
        (
          WidgetInstance::Render(render),
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
        self.track_widget(new_id);
        if let Some(c_children) = c_children {
          stack.push((parent, children));
          stack.push((new_id, c_children));
          break;
        }
      }
    }
  }

  fn node_to_key(&self, wid: NodeId) -> Option<Key> {
    if let WidgetInstance::Render(ref r) = self.w_arena.get(wid)?.get().widget {
      r.as_any()?.downcast_ref::<Key>().map(|k| k.clone())
    } else {
      None
    }
  }

  fn down_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let WidgetInstance::Combination(_) = self.w_arena[wid].get().widget {
      // combination widget always have single child.
      debug_assert_eq!(wid.children(&self.w_arena).count(), 1);

      wid = self.w_arena[wid]
        .first_child()
        .expect("Combination node must be only one child")
    }
    debug_assert!(matches!(
      &self.w_arena[wid].get().widget,
      WidgetInstance::Render(_)
    ));

    wid
  }

  fn upper_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let WidgetInstance::Combination(_) = self.w_arena[wid].get().widget {
      wid = self.w_arena[wid].parent().expect(
        "should only call this method if `wid`  have render widget ancestor!",
      );
    }
    debug_assert!(matches!(
      &self.w_arena[wid].get().widget,
      WidgetInstance::Render(_)
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
    let render_object = if let WidgetInstance::Render(ref r) =
      self.w_arena[render_wid].get().widget
    {
      r.create_render_object()
    } else {
      unreachable!("only render widget can create render object!");
    };
    render_object
  }

  fn repair_tree(&mut self) {
    while let Some(first) = self.wait_rebuilds.iter().nth(0).map(|id| *id) {
      // Always find the topmost widget which need to rebuild to rebuild
      // subtree.
      if let Some(top) = self.get_rebuild_ancestors(first) {
        if let Some(sub_root) = self.w_arena.get_mut(top) {
          debug_assert!(
            matches!(sub_root.get().widget, WidgetInstance::Combination(_)),
            "rebuild widget must be combination widget."
          );

          // combination widget should have only one child."
          debug_assert!(sub_root.first_child().is_some());
          debug_assert_eq!(sub_root.first_child(), sub_root.last_child());

          if let WidgetInstance::Combination(ref c) = sub_root.get().widget {
            let new_widget = c.build();
            let old_node =
              sub_root.first_child().expect("should have single child");

            self.repair_subtree(old_node, new_widget);
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
  fn repair_subtree(&mut self, old_node_id: NodeId, new_widget: Widget) {
    /// Detect if the `new_widget` is a same widget with the `node` by `Key`.
    fn same_widget(
      node: &Node<WidgetNode>,
      new_widget: &Box<dyn for<'r> RenderWidget<'r>>,
    ) -> bool {
      fn option_same(
        node: &Node<WidgetNode>,
        new_widget: &Box<dyn for<'r> RenderWidget<'r>>,
      ) -> Option<bool> {
        let old_key = node.get().widget.as_any()?.downcast_ref::<Key>()?;
        let new_key = new_widget.as_any()?.downcast_ref::<Key>()?;
        Some(old_key == new_key)
      }
      option_same(node, new_widget).map_or(false, |v| v)
    }

    if let Widget::SingleChild(single) = new_widget {
      let (r, w) = single.split();
      if same_widget(&self.w_arena[old_node_id], &r) {
        self.wait_rebuilds.remove(&old_node_id);
        debug_assert_eq!(old_node_id.children(&self.w_arena).count(), 1);
        let content_id = self.w_arena[old_node_id]
          .first_child()
          .expect("should have single child");

        // keep node, but replace widget in node with new widget.
        let (w, children) = Self::consume_widget_to_node(w);
        self.w_arena[content_id].get_mut().widget = w;
        self.dirty_widgets.insert(content_id);

        if let Some(widgets) = children {
          let mut key_children = HashMap::new();
          let mut child = self.w_arena[content_id].first_child();
          while let Some(id) = child {
            child = self.w_arena[id].next_sibling();
            if let Some(key) = self.node_to_key(id) {
              id.detach(&mut self.w_arena);
              key_children.insert(key, id);
              debug_assert!(
                self.tree_relationship.widget_to_render(id).is_some(),
                format!("{:?} not have correspond render object", id)
              );
            } else {
              self.drop_subtree(id);
            }
          }

          for w in widgets.into_iter() {
            if let Some(k) = w.key() {
              if let Some(id) = key_children.get(k).map(|id| *id) {
                key_children.remove(k);
                content_id.append(id, &mut self.w_arena);
                self.repair_subtree(id, w);
                continue;
              }
            }
            let (w, children) = Self::consume_widget_to_node(w);
            let child_id = self.append_widget(content_id, w);
            if let Some(children) = children {
              self.inflate_widget_subtree(child_id, children);
              self.construct_render_tree(child_id);
            }
          }

          key_children.into_iter().for_each(|(_, v)| {
            self.drop_subtree(v);
          });
        } else {
          // There is not children in new widget, drop all old children.
          let mut child = self.w_arena[content_id].first_child();
          while let Some(c) = child {
            child = self.w_arena[c].next_sibling();
            self.drop_subtree(c);
          }
        }
      } else {
        self.rebuild_subtree(
          old_node_id,
          WidgetInstance::Render(r),
          Some(smallvec![w]),
        );
      }
    } else {
      let (w, children) = Self::consume_widget_to_node(new_widget);
      self.rebuild_subtree(old_node_id, w, children);
    }
  }

  fn rebuild_subtree(
    &mut self,
    wid: NodeId,
    new_widget: WidgetInstance,
    children: Option<SmallVec<[Widget; 1]>>,
  ) {
    let parent_id = self.w_arena[wid].parent().expect("parent should exists!");
    self.drop_subtree(wid);

    let new_child_id = self.append_widget(parent_id, new_widget);
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
      if matches!(w_arena[id].get().widget, WidgetInstance::Render(_)) {
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

  fn append_widget(&mut self, wid: NodeId, w: WidgetInstance) -> NodeId {
    let child = self.w_arena.new_node(WidgetNode::new(w));
    wid.append(child, &mut self.w_arena);
    child
  }

  fn preappend_widget(&mut self, wid: NodeId, w: WidgetInstance) -> NodeId {
    let child = self.w_arena.new_node(WidgetNode::new(w));
    wid.prepend(child, &mut self.w_arena);
    child
  }

  fn track_widget(&mut self, wid: NodeId) {
    let mut node = self.w_arena[wid].get_mut();

    debug_assert!(node.subscription_guards.0.is_none());
    debug_assert!(node.subscription_guards.1.is_none());

    let mut node_ptr: NonNull<_> = (&mut self.dirty_widgets).into();
    node.subscription_guards.0 =
      node.widget.changed_emitter(self.notifier.clone()).map(|e| {
        // Safety: framework logic promise the `node_ptr` always valid.
        e.subscribe(move |_| unsafe {
          node_ptr.as_mut().insert(wid);
        })
        .unsubscribe_when_dropped()
      });

    if let WidgetInstance::Combination(c) = &mut node.widget {
      let mut node_ptr: NonNull<_> = (&mut self.wait_rebuilds).into();
      node.subscription_guards.1 = c
        .rebuild_emitter(self.notifier.clone())
        // Safety: framework logic promise the `node_ptr` always valid.
        .map(|e| {
          e.subscribe(move |_| unsafe {
            node_ptr.as_mut().insert(wid);
          })
          .unsubscribe_when_dropped()
        });
    }
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

use std::fmt::{Debug, Formatter, Result};
impl Debug for WidgetNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { self.widget.fmt(f) }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::widget::Row;
  use crate::{render_object_box::*, render_ctx::*};
  extern crate test;
  use test::Bencher;

  #[derive(Clone, Debug)]
  struct EmbedPost {
    title: &'static str,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl From<EmbedPost> for Widget {
    fn from(c: EmbedPost) -> Self { Widget::Combination(Box::new(c)) }
  }

  #[derive(Debug)]
  struct RowRenderObject;

  impl RenderObject for RowRenderObject {
    fn paint(&self) {}
    fn perform_layout(&mut self, _ctx: RenderCtx) {}
  }

  #[derive(Debug)]
  struct RenderRow {}

  impl<'a> WidgetStates<'a> for RenderRow {}
  impl<'a> RenderWidget<'a> for RenderRow {
    fn create_render_object(&self) -> Box<dyn RenderObject> {
      Box::new(RowRenderObject {})
    }
  }

  impl From<RenderRow> for Widget {
    fn from(r: RenderRow) -> Self { Widget::Render(Box::new(r)) }
  }

  struct Row {
    children: Vec<Widget>,
  }

  impl From<Row> for Widget {
    fn from(r: Row) -> Self { Widget::MultiChild(Box::new(r)) }
  }

  impl<'a> WidgetStates<'a> for Row {}
  impl<'a> MultiChildWidget<'a> for Row {
    fn split(
      self: Box<Self>,
    ) -> (Box<dyn for<'r> RenderWidget<'r>>, Vec<Widget>) {
      (Box::new(RenderRow {}), self.children)
    }
  }

  impl<'a> WidgetStates<'a> for EmbedPost {}
  impl<'a> CombinationWidget<'a> for EmbedPost {
    fn build(&self) -> Widget {
      let mut row = Row {
        children: vec![
          Text(self.title).into(),
          Text(self.author).into(),
          Text(self.content).into(),
        ],
      };
      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        row.children.push(embed.into())
      }
      row.into()
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
    app.inflate(post.into());
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
      r#"RowRenderObject
├── Text("Simple demo")
├── Text("Adoo")
├── Text("Recursive x times")
└── RowRenderObject
    ├── Text("Simple demo")
    ├── Text("Adoo")
    ├── Text("Recursive x times")
    └── RowRenderObject
        ├── Text("Simple demo")
        ├── Text("Adoo")
        ├── Text("Recursive x times")
        └── RowRenderObject
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
    rebuild_emitter: LocalSubject<'static, (), ()>,
  }

  use std::any::Any;
  impl<'a> WidgetStates<'a> for EmbedKeyPost {
    fn as_any(&self) -> Option<&dyn Any> { Some(&*self) }

    fn changed_emitter(
      &mut self,
      _notifier: LocalSubject<'a, (), ()>,
    ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
      let res: LocalCloneBoxOp<'static, (), ()> =
        self.rebuild_emitter.clone().box_it();
      // not a good code below, just use for test.
      unsafe { std::mem::transmute(res) }
    }
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

  impl From<EmbedKeyPost> for Widget {
    fn from(c: EmbedKeyPost) -> Self { Widget::Combination(Box::new(c)) }
  }

  impl<'a> CombinationWidget<'a> for EmbedKeyPost {
    fn build(&self) -> Widget {
      let mut row = Row {
        children: vec![
          KeyDetect::new(0, Text(*self.title.borrow())).into(),
          KeyDetect::new(1, Text(self.author)).into(),
          KeyDetect::new(2, Text(self.content)).into(),
        ],
      };
      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        row.children.push(KeyDetect::new("embed", embed).into())
      }
      KeyDetect::new(0, row).into()
    }
  }

  fn key_detect_env<'a>(level: usize) -> Application<'a> {
    let mut post = EmbedKeyPost::default();
    post.level = level;
    let title = post.title.clone();
    let mut emitter = post.rebuild_emitter.clone();

    let mut app = Application::new();
    app.inflate(post.clone().into());
    app.construct_render_tree(app.widget_tree.unwrap());

    *title.borrow_mut() = "New title";
    emitter.next(());

    app
  }

  #[test]
  fn repair_tree() {
    let mut app = key_detect_env(3);
    app.repair_tree();

    assert_eq!(
      app.widget_symbol_tree(),
      r#"Combination(EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 3 })
└── Render(KI4(0))
    └── Render(RenderRow)
        ├── Render(KI4(0))
        │   └── Render(Text("New title"))
        ├── Render(KI4(1))
        │   └── Render(Text(""))
        ├── Render(KI4(2))
        │   └── Render(Text(""))
        └── Render(KString("embed"))
            └── Combination(EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 2 })
                └── Render(KI4(0))
                    └── Render(RenderRow)
                        ├── Render(KI4(0))
                        │   └── Render(Text("New title"))
                        ├── Render(KI4(1))
                        │   └── Render(Text(""))
                        ├── Render(KI4(2))
                        │   └── Render(Text(""))
                        └── Render(KString("embed"))
                            └── Combination(EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 1 })
                                └── Render(KI4(0))
                                    └── Render(RenderRow)
                                        ├── Render(KI4(0))
                                        │   └── Render(Text("New title"))
                                        ├── Render(KI4(1))
                                        │   └── Render(Text(""))
                                        ├── Render(KI4(2))
                                        │   └── Render(Text(""))
                                        └── Render(KString("embed"))
                                            └── Combination(EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 0 })
                                                └── Render(KI4(0))
                                                    └── Render(RenderRow)
                                                        ├── Render(KI4(0))
                                                        │   └── Render(Text("New title"))
                                                        ├── Render(KI4(1))
                                                        │   └── Render(Text(""))
                                                        └── Render(KI4(2))
                                                            └── Render(Text(""))
"#
    );

    assert_eq!(app.render_symbol_tree(), 
r#"KeyRender
└── RowRenderObject
    ├── KeyRender
    │   └── Text("New title")
    ├── KeyRender
    │   └── Text("")
    ├── KeyRender
    │   └── Text("")
    └── KeyRender
        └── KeyRender
            └── RowRenderObject
                ├── KeyRender
                │   └── Text("New title")
                ├── KeyRender
                │   └── Text("")
                ├── KeyRender
                │   └── Text("")
                └── KeyRender
                    └── KeyRender
                        └── RowRenderObject
                            ├── KeyRender
                            │   └── Text("New title")
                            ├── KeyRender
                            │   └── Text("")
                            ├── KeyRender
                            │   └── Text("")
                            └── KeyRender
                                └── KeyRender
                                    └── RowRenderObject
                                        ├── KeyRender
                                        │   └── Text("New title")
                                        ├── KeyRender
                                        │   └── Text("")
                                        └── KeyRender
                                            └── Text("")
"#);
  }

  fn assert_root_bound(app:&mut Application, bound: Option<Size>) {
    let mut mut_root = app.render_tree.root_mut().unwrap();
    let render_box = mut_root.data().to_render_box().unwrap();
    assert_eq!(render_box.bound(), bound);
  }

  fn layout_app(app:&mut Application) {
    let mut_ptr = &mut app.render_tree as *mut Tree<Box<dyn RenderObject>>;
    let root_id = app.render_tree.root().unwrap().node_id();
    unsafe {
        app.render_tree.root_mut().unwrap().data().layout(root_id, &mut RenderCtx::new(&mut *mut_ptr));
    } 
  }

  fn mark_dirty(app: &mut Application, node_id: NodeId) {
    let mut_ptr = &mut app.render_tree as *mut Tree<Box<dyn RenderObject>>;
    unsafe {
        app.render_tree.get_mut(node_id).unwrap().data().mark_dirty(node_id, &mut RenderCtx::new(&mut *mut_ptr));
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
      app.inflate(post.into());
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
    app.inflate(post.into());
    app.construct_render_tree();
    
    layout_app(&mut app);
    assert_root_bound(&mut app, Some(Size{width: 192, height: 1}));

    let last_child_id = app.render_tree.root().unwrap().last_child().unwrap().node_id();
    mark_dirty(&mut app, last_child_id);
    assert_root_bound(&mut app, None);
    
    layout_app(&mut app);
    assert_root_bound(&mut app, Some(Size{width: 192, height: 1}));
  }

  #[bench]
  fn widget_tree_deep_1000_with_key(b: &mut Bencher) {
    b.iter(|| {
      let mut app = key_detect_env(50);
      app.repair_tree();
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
      app.inflate(post.into());
      app.construct_render_tree(app.widget_tree.expect("must exists"));
    });
  }
}
