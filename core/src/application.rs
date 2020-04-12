use crate::{render::*, util::TreeFormatter, widget::*};
use indextree::*;
use std::collections::{HashMap, HashSet};
mod tree_relationship;
use tree_relationship::Relationship;

#[derive(Default)]
pub struct Application<'a> {
  w_arena: Arena<Widget<'a>>,
  render_tree: RenderTree,
  widget_tree: Option<NodeId>,
  tree_relationship: Relationship,
  /// Store widgets that modified and wait to update its corresponds render
  /// object in render tree.
  dirty_widgets: HashSet<NodeId>,
  /// Store combination widgets that has require to rebuild its subtree.
  wait_rebuilds: HashSet<NodeId>,

  dirty_layouts: HashSet<NodeId>,
  dirty_layout_roots: HashSet<NodeId>,
}

impl<'a> Application<'a> {
  #[inline]
  pub fn new() -> Application<'a> { Default::default() }

  pub fn run(mut self, w: Widget<'a>) {
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
  fn inflate(&mut self, w: Widget<'a>) {
    let root = self.w_arena.new_node(w);
    self.widget_tree = Some(root);

    self.inflate_widget_subtree(root);
  }

  /// Return an widget after inflated, and its children

  fn inflate_widget_subtree(&mut self, sub_tree: NodeId) {
    let mut stack = vec![sub_tree];

    fn append<'a>(
      parent: NodeId,
      widget: Widget<'a>,
      stack: &mut Vec<NodeId>,
      tree: &mut Application<'a>,
    ) {
      let node = tree.append_widget(parent, widget);
      stack.push(node);
    }

    while let Some(parent) = stack.pop() {
      let p_widget = self.w_arena[parent].get_mut();
      match p_widget {
        Widget::Combination(ref c) => {
          append(parent, c.build(), &mut stack, self);
        }
        Widget::SingleChild(single) => {
          append(parent, single.take_child(), &mut stack, self);
        }
        Widget::MultiChild(multi) => {
          multi.take_children().into_iter().for_each(|w| {
            append(parent, w, &mut stack, self);
          });
        }
        Widget::Render(_) => {
          // Touch leaf, nothing to do.
        }
      }
    }
  }

  fn down_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let Widget::Combination(_) = self.w_arena[wid].get() {
      wid = self.assert_widget_single_child(wid);
    }
    debug_assert!(!matches!(&self.w_arena[wid].get(), Widget::Combination(_)));

    wid
  }

  fn upper_to_render_widget(&self, mut wid: NodeId) -> NodeId {
    while let Widget::Combination(_) = self.w_arena[wid].get() {
      wid = self.w_arena[wid].parent().expect(
        "should only call this method if `wid`  have render widget ancestor!",
      );
    }
    debug_assert!(matches!(&self.w_arena[wid].get(), Widget::Render(_)));

    wid
  }

  /// construct a render tree correspond to widget tree `wid`.
  fn construct_render_tree(&mut self, wid: NodeId) {
    let (r_wid, rid) = self.widget_render_pair(wid);

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

  /// Return a pair of (render widget node id, render object node id) from the
  /// widget node id `wid`, if a render object node exist, will create it.
  fn widget_render_pair(&mut self, wid: NodeId) -> (NodeId, RenderId) {
    let mut r_wid = self.down_to_render_widget(wid);
    if self.render_tree.root().is_none() {
      let rid = self.render_tree.new_node(self.create_render_object(r_wid));
      self.render_tree.set_root(rid);
      self.tree_relationship.bind(r_wid, rid);
    }

    if let Some(render_id) = self.tree_relationship.widget_to_render(r_wid) {
      (r_wid, *render_id)
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
      (r_wid, render_object)
    }
  }

  fn render_tree_depth_construct(
    &mut self,
    mut wid: NodeId,
    mut rid: RenderId,
    stack: &mut Vec<(NodeId, RenderId)>,
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
    rid: RenderId,
  ) -> (NodeId, RenderId) {
    wid = self.down_to_render_widget(wid);
    let r_child = self.render_tree.new_node(self.create_render_object(wid));
    rid.append(r_child, &mut self.render_tree);
    self.tree_relationship.bind(wid, r_child);
    (wid, r_child)
  }

  fn create_render_object(
    &self,
    render_wid: NodeId,
  ) -> Box<dyn RenderObjectSafety + Send + Sync> {
    match self.w_arena[render_wid].get() {
      Widget::Combination(_) => {
        unreachable!("only render widget can create render object!")
      }
      Widget::Render(r) => r.create_render_object(),
      Widget::SingleChild(r) => r.create_render_object(),
      Widget::MultiChild(r) => r.create_render_object(),
    }
  }

  fn repair_tree(&mut self) {
    while let Some(first) = self.wait_rebuilds.iter().nth(0).map(|id| *id) {
      // Always find the topmost widget which need to rebuild to rebuild
      // subtree.
      let top = self.get_rebuild_ancestors(first);
      let widget = self.w_arena[top].get_mut();

      debug_assert!(
        matches!(widget, Widget::Combination(_)),
        "rebuild widget must be combination widget."
      );

      if let Widget::Combination(ref c) = widget {
        let new_widget = c.build();
        let old_node = self.assert_widget_single_child(top);
        self.repair_subtree(old_node, new_widget);
        self.wait_rebuilds.remove(&top);
      }
    }
  }

  fn repair_subtree(&mut self, old_node: NodeId, new_widget: Widget<'a>) {
    let mut stack = vec![(old_node, new_widget)];

    while let Some((old_node, new_widget)) = stack.pop() {
      let old_key = self.w_arena[old_node].get().key();
      if old_key.is_some() && old_key == new_widget.key() {
        debug_assert!(
          new_widget.same_type_widget(self.w_arena[old_node].get())
        );
        self.replace_widget(old_node, new_widget, &mut stack)
      } else {
        self.rebuild_subtree(old_node, new_widget);
      }
      self.wait_rebuilds.remove(&old_node);
    }
  }

  /// rebuild the subtree `wid` by the new children `new_children`, the same key
  /// children as before will keep the old subtree and will add into the `stack`
  /// to recursive repair, else will construct a new subtree.
  fn repair_children_by_key(
    &mut self,
    wid: NodeId,
    new_children: Vec<Widget<'a>>,
    stack: &mut Vec<(NodeId, Widget<'a>)>,
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
          self.replace_widget(id, w, stack);
          continue;
        }
      }

      let child_id = self.append_widget(wid, w);

      self.inflate_widget_subtree(child_id);
      self.construct_render_tree(child_id);
    }

    key_children.into_iter().for_each(|(_, v)| {
      self.drop_subtree(v);
    });
  }

  fn replace_widget(
    &mut self,
    old_node: NodeId,
    mut new_widget: Widget<'a>,
    stack: &mut Vec<(NodeId, Widget<'a>)>,
  ) {
    match new_widget {
      Widget::Combination(ref c) => {
        let new_child = c.build();
        let old_child_node = self.assert_widget_single_child(old_node);
        stack.push((old_child_node, new_child));
      }
      Widget::SingleChild(ref mut r) => {
        let new_child = r.take_child();
        let old_child_node = self.assert_widget_single_child(old_node);
        stack.push((old_child_node, new_child));
      }
      Widget::MultiChild(ref mut multi) => {
        let children = multi.take_children();
        self.repair_children_by_key(old_node, children, stack);
      }
      Widget::Render(_) => {
        // down to leaf, nothing to do.
      }
    }
    *self.w_arena[old_node].get_mut() = new_widget;
    self.dirty_widgets.insert(old_node);
  }

  fn rebuild_subtree(&mut self, old_node: NodeId, new_widget: Widget<'a>) {
    let parent_id = self.w_arena[old_node]
      .parent()
      .expect("parent should exists!");
    self.drop_subtree(old_node);

    let new_child_id = self.append_widget(parent_id, new_widget);

    self.inflate_widget_subtree(new_child_id);

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
      if !matches!(w_arena[id].get(), Widget::Combination(_)) {
        tree_relationship.unbind(id)
      }
      dirty_widgets.remove(&id);
      wait_rebuilds.remove(&id);
    });

    // Todo: should remove in a more directly way and not care about
    // relationship
    // Fixme: memory leak here, node not remove.
    wid.detach(&mut self.w_arena);
    rid.detach(&mut self.render_tree);
    if self.widget_tree == Some(wid) {
      self.widget_tree = None;
    }
  }

  fn get_rebuild_ancestors(&self, wid: NodeId) -> NodeId {
    wid
      .ancestors(&self.w_arena)
      .filter(|id| self.wait_rebuilds.contains(id))
      .last()
      .unwrap_or(wid)
  }

  fn append_widget(&mut self, wid: NodeId, w: Widget<'a>) -> NodeId {
    let child = self.w_arena.new_node(w);
    wid.append(child, &mut self.w_arena);
    child
  }

  fn assert_widget_single_child(&self, wid: NodeId) -> NodeId {
    debug_assert!(self.w_arena[wid].first_child().is_some());
    debug_assert_eq!(
      self.w_arena[wid].first_child(),
      self.w_arena[wid].last_child()
    );
    self.w_arena[wid]
      .first_child()
      .expect("Caller assert `wid` has single child")
  }

  #[allow(dead_code)]
  pub(crate) fn widget_symbol_tree(&self) -> String {
    if let Some(w_root) = self.widget_tree {
      format!("{:?}", TreeFormatter::new(&self.w_arena, w_root))
    } else {
      "".to_owned()
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::widget::Row;
  use crate::{render_ctx::*, render_object_box::*};
  use std::fmt::{Debug, Formatter, Result};
  extern crate test;
  use test::Bencher;

  #[derive(Clone, Debug)]
  struct EmbedPost {
    title: &'static str,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl CombinationWidget for EmbedPost {
    fn build<'a>(&self) -> Widget<'a> {
      let mut children = vec![
        Text(self.title).to_widget(),
        Text(self.author).to_widget(),
        Text(self.content).to_widget(),
      ];

      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        children.push(embed.to_widget())
      }
      Row::new(children).to_widget()
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
└── MultiChild(Row { children: None })
    ├── Render(Text("Simple demo"))
    ├── Render(Text("Adoo"))
    ├── Render(Text("Recursive x times"))
    └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 2 })
        └── MultiChild(Row { children: None })
            ├── Render(Text("Simple demo"))
            ├── Render(Text("Adoo"))
            ├── Render(Text("Recursive x times"))
            └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 1 })
                └── MultiChild(Row { children: None })
                    ├── Render(Text("Simple demo"))
                    ├── Render(Text("Adoo"))
                    ├── Render(Text("Recursive x times"))
                    └── Combination(EmbedPost { title: "Simple demo", author: "Adoo", content: "Recursive x times", level: 0 })
                        └── MultiChild(Row { children: None })
                            ├── Render(Text("Simple demo"))
                            ├── Render(Text("Adoo"))
                            └── Render(Text("Recursive x times"))
"#
    );

    assert_eq!(
      app.render_tree.symbol_shape(),
      r#"RowRender { inner_layout: [], size: None }
├── TextRender("Simple demo")
├── TextRender("Adoo")
├── TextRender("Recursive x times")
└── RowRender { inner_layout: [], size: None }
    ├── TextRender("Simple demo")
    ├── TextRender("Adoo")
    ├── TextRender("Recursive x times")
    └── RowRender { inner_layout: [], size: None }
        ├── TextRender("Simple demo")
        ├── TextRender("Adoo")
        ├── TextRender("Recursive x times")
        └── RowRender { inner_layout: [], size: None }
            ├── TextRender("Simple demo")
            ├── TextRender("Adoo")
            └── TextRender("Recursive x times")
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
    assert!(app.render_tree.root().is_none());
  }

  use std::{cell::RefCell, rc::Rc};
  #[derive(Clone, Default, Debug)]
  struct EmbedKeyPost {
    title: Rc<RefCell<&'static str>>,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl CombinationWidget for EmbedKeyPost {
    fn build<'a>(&self) -> Widget<'a> {
      let mut children = vec![
        KeyDetect::new(0, Text(*self.title.borrow())).to_widget(),
        KeyDetect::new(1, Text(self.author)).to_widget(),
        KeyDetect::new(2, Text(self.content)).to_widget(),
      ];

      if self.level > 0 {
        let mut embed = self.clone();
        embed.level -= 1;
        children.push(KeyDetect::new("embed", embed).to_widget())
      }
      KeyDetect::new(0, Row::new(children)).to_widget()
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
└── MultiChild(KeyDetect { key: KI4(0), child: Row { children: None } })
    ├── Render(KeyDetect { key: KI4(0), child: Text("") })
    ├── Render(KeyDetect { key: KI4(1), child: Text("") })
    ├── Render(KeyDetect { key: KI4(2), child: Text("") })
    └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 2 } })
        └── MultiChild(KeyDetect { key: KI4(0), child: Row { children: None } })
            ├── Render(KeyDetect { key: KI4(0), child: Text("") })
            ├── Render(KeyDetect { key: KI4(1), child: Text("") })
            ├── Render(KeyDetect { key: KI4(2), child: Text("") })
            └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 1 } })
                └── MultiChild(KeyDetect { key: KI4(0), child: Row { children: None } })
                    ├── Render(KeyDetect { key: KI4(0), child: Text("") })
                    ├── Render(KeyDetect { key: KI4(1), child: Text("") })
                    ├── Render(KeyDetect { key: KI4(2), child: Text("") })
                    └── Combination(KeyDetect { key: KString("embed"), child: EmbedKeyPost { title: RefCell { value: "New title" }, author: "", content: "", level: 0 } })
                        └── MultiChild(KeyDetect { key: KI4(0), child: Row { children: None } })
                            ├── Render(KeyDetect { key: KI4(0), child: Text("") })
                            ├── Render(KeyDetect { key: KI4(1), child: Text("") })
                            └── Render(KeyDetect { key: KI4(2), child: Text("") })
"#
    );

    // fixme: below assert should failed, after support update render tree.
    assert_eq!(
      env.app.render_tree.symbol_shape(),
r#"KeyRender(RowRender { inner_layout: [], size: None })
├── KeyRender(TextRender(""))
├── KeyRender(TextRender(""))
├── KeyRender(TextRender(""))
└── KeyRender(RowRender { inner_layout: [], size: None })
    ├── KeyRender(TextRender(""))
    ├── KeyRender(TextRender(""))
    ├── KeyRender(TextRender(""))
    └── KeyRender(RowRender { inner_layout: [], size: None })
        ├── KeyRender(TextRender(""))
        ├── KeyRender(TextRender(""))
        ├── KeyRender(TextRender(""))
        └── KeyRender(RowRender { inner_layout: [], size: None })
            ├── KeyRender(TextRender(""))
            ├── KeyRender(TextRender(""))
            └── KeyRender(TextRender(""))
"#
    );
  }

  // fn assert_root_bound(app: &mut Application, bound: Option<Size>) {
  //   let root = app.r_arena.get_mut(app.render_tree.unwrap()).unwrap();
  //   let render_box = root.get_mut().to_render_box().unwrap();
  //   assert_eq!(render_box.bound(), bound);
  // }

  // fn layout_app(app: &mut Application) {
  //   let mut_ptr = &mut app.r_arena as *mut Arena<Box<(dyn RenderObject + Send + Sync)>>;
  //   let mut ctx = RenderCtx::new(&mut app.r_arena, &mut app.dirty_layouts, &mut app.dirty_layout_roots);
  //   unsafe {
  //       let root = mut_ptr.as_mut().unwrap().get_mut(app.render_tree.unwrap()).unwrap();
  //       root.get_mut().perform_layout(app.render_tree.unwrap(), &mut ctx);
  //   }
  // }

  // fn mark_dirty(app: &mut Application, node_id: NodeId) {
  //   let mut_ptr = &mut app.r_arena as *mut Arena<Box<(dyn RenderObject + Send + Sync)>>;
  //   let mut ctx = RenderCtx::new(&mut app.r_arena, &mut app.dirty_layouts, &mut app.dirty_layout_roots);
    
  //   unsafe {
  //      mut_ptr
  //       .as_mut()
  //       .unwrap()
  //       .get_mut(node_id)
  //       .unwrap()
  //       .get_mut()
  //       .mark_dirty(node_id, &mut ctx);
  //   }
  // }

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

  // #[test]
  // fn test_layout() {
  //   let post = EmbedPost {
  //     title: "Simple demo",
  //     author: "Adoo",
  //     content: "Recursive 5 times",
  //     level: 5,
  //   };
  //   let mut app = Application::new();
  //   app.inflate(post.to_widget());
  //   app.construct_render_tree(app.widget_tree.unwrap());

  //   let root_id = app.render_tree.unwrap();
  //   mark_dirty(&mut app, root_id);
  //   layout_app(&mut app);
  //   assert_root_bound(
  //     &mut app,
  //     Some(Size {
  //       width: 192,
  //       height: 1,
  //     }),
  //   );

  //   let last_child_id = app
  //     .r_arena
  //     .get(app.render_tree.unwrap())
  //     .unwrap()
  //     .last_child()
  //     .unwrap();
  //   mark_dirty(&mut app, last_child_id);
  //   assert_eq!(app.dirty_layouts.contains(&root_id), true);

  //   layout_app(&mut app);
  //   assert_eq!(app.dirty_layouts.contains(&root_id), false);
  //   assert_root_bound(
  //     &mut app,
  //     Some(Size {
  //       width: 192,
  //       height: 1,
  //     }),
  //   );
  // }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let mut env = KeyDetectEnv::default();
    env.construct_tree(1000);
    b.iter(|| {
      env.emit_rebuild();
      env.app.repair_tree();
    });
  }

  #[bench]
  fn render_tree_5_x_1000(b: &mut Bencher) {
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
