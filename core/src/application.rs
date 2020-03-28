use crate::{render_object::*, widget::*};
use ::herald::prelude::*;
use slab_tree::*;
use std::{
  collections::{HashMap, HashSet},
  ptr::NonNull,
};

enum WidgetInstance {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
}

struct WidgetNode {
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
  widget_tree: Tree<WidgetNode>,
  render_tree: Tree<Box<dyn RenderObject>>,
  /// A hash map to mapping a render widget in widget tree to its corresponds
  /// render object in render tree.
  widget_to_render: HashMap<NodeId, NodeId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<NodeId, NodeId>,
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
    self.construct_render_tree();

    todo!("not rebuild widget tree & render tree when change occurs");
    self.update_widget_tree();
  }

  /// inflate widget tree, so every widget tree leaf should be a render object.
  fn inflate(&mut self, w: Widget) {
    enum StackElem {
      Widget(Widget),
      NodeID(NodeId),
    }

    /// Return an widget after inflated, and store the sub widgets into the
    /// `stack`
    #[inline]
    fn inflate_widget(
      widget: Widget,
      stack: &mut Vec<StackElem>,
    ) -> WidgetInstance {
      match widget {
        Widget::Combination(w) => {
          let c = w.build();
          stack.push(StackElem::Widget(c));
          WidgetInstance::Combination(w)
        }
        Widget::Render(r) => WidgetInstance::Render(r),
        Widget::SingleChild(w) => {
          let (render, child) = w.split();
          stack.push(StackElem::Widget(child));
          WidgetInstance::Render(render)
        }
        Widget::MultiChild(w) => {
          let (render, children) = w.split();
          children
            .into_iter()
            .rev()
            .for_each(|w| stack.push(StackElem::Widget(w)));
          WidgetInstance::Render(render)
        }
      }
    }

    let mut stack = vec![];
    let widget_node = inflate_widget(w, &mut stack);
    let mut node_id = self.widget_tree.set_root(WidgetNode::new(widget_node));
    self.track_widget(node_id);

    while let Some(elem) = stack.pop() {
      match elem {
        StackElem::NodeID(id) => node_id = id,
        StackElem::Widget(widget) => {
          stack.push(StackElem::NodeID(node_id));

          let widget_node = inflate_widget(widget, &mut stack);
          let new_id = self.append_widget_by_id(node_id, widget_node);

          stack.push(StackElem::NodeID(new_id));
          self.track_widget(new_id);
        }
      }
    }
  }

  /// construct a full render tree correspond to widget tree.
  fn construct_render_tree(&mut self) {
    fn skip_to_render_widget(
      mut node: NodeRef<WidgetNode>,
    ) -> (NodeId, Box<dyn RenderObject>) {
      while let WidgetInstance::Combination(_) = &node.data().widget {
        debug_assert!(node.first_child().unwrap().next_sibling().is_none());
        let child = node
          .first_child()
          .expect("Combination node must be only one child");
        // Safety: child's lifetime is bind to widget_tree not a NodeRef temp
        // variable.
        node = unsafe { std::mem::transmute(child) };
      }
      debug_assert!(matches!(&node.data().widget, WidgetInstance::Render(_)));

      let render_object =
        if let WidgetInstance::Render(ref r) = node.data().widget {
          r.create_render_object()
        } else {
          unreachable!("only render widget can create render object!");
        };
      (node.node_id(), render_object)
    }

    let Self {
      widget_to_render,
      render_to_widget,
      widget_tree,
      render_tree,
      ..
    } = self;

    let (w_id, render_obj) =
      skip_to_render_widget(widget_tree.root().expect("root must exist!"));
    let r_id = render_tree.set_root(render_obj);
    Self::bind_widget_and_render(
      widget_to_render,
      render_to_widget,
      w_id,
      r_id,
    );

    let mut w_stack = vec![w_id];

    while let Some(w_id) = w_stack.pop() {
      widget_tree
        .get(w_id)
        .expect("must have")
        .children()
        .for_each(|w_child_node| {
          let (w_child_id, r) = skip_to_render_widget(w_child_node);
          w_stack.push(w_child_id);

          let r_id = *widget_to_render.get(&w_id).unwrap();
          let mut r_node = render_tree.get_mut(r_id).unwrap();
          let r_child_id = r_node.append(r).node_id();
          Self::bind_widget_and_render(
            widget_to_render,
            render_to_widget,
            w_child_id,
            r_child_id,
          );
        });
    }
  }

  fn update_widget_tree(&mut self) {
    while let Some(first) = self.wait_rebuilds.iter().nth(0).map(|id| *id) {
      // Always find the topmost rebuild widget to rebuild subtree.
      if let Some(top) = self.get_rebuild_ancestors(first) {
        self.rebuild_subtree(top);
      } else {
        self.wait_rebuilds.remove(&first);
      }
    }
  }

  fn rebuild_subtree(&mut self, id: NodeId) {
    let mut sub_root = self.widget_tree.get_mut(id).unwrap();
    if let WidgetInstance::Combination(ref c) = sub_root.data().widget {
      todo!("rebuild subtree depends on widget key");
    } else {
      debug_assert!(false, "rebuild widget must be combination widget.");
    }
  }

  fn get_rebuild_ancestors(&self, id: NodeId) -> Option<NodeId> {
    let node = self.widget_tree.get(id);
    node?
      .ancestors()
      .map(|node| node.node_id())
      .filter(|id| self.wait_rebuilds.contains(id))
      .last()
      .or(Some(id))
  }

  fn append_widget_by_id(&mut self, id: NodeId, w: WidgetInstance) -> NodeId {
    let mut node = self
      .widget_tree
      .get_mut(id)
      .expect("node have to exist in logic");

    node.append(WidgetNode::new(w)).node_id()
  }

  fn track_widget(&mut self, id: NodeId) {
    let mut w = self.widget_tree.get_mut(id).unwrap();
    let node = w.data();
    debug_assert!(node.subscription_guards.0.is_none());
    debug_assert!(node.subscription_guards.1.is_none());

    let mut node_ptr: NonNull<_> = (&mut self.dirty_widgets).into();
    node.subscription_guards.0 =
      node.widget.changed_emitter(self.notifier.clone()).map(|e| {
        // Safety: framework logic promise the `node_ptr` always valid.
        e.subscribe(move |_| unsafe {
          node_ptr.as_mut().insert(id);
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
            node_ptr.as_mut().insert(id);
          })
          .unsubscribe_when_dropped()
        });
    }
  }

  fn bind_widget_and_render(
    w_2_r: &mut HashMap<NodeId, NodeId>,
    r_2_w: &mut HashMap<NodeId, NodeId>,
    w_id: NodeId,
    r_id: NodeId,
  ) {
    w_2_r.insert(w_id, r_id);
    r_2_w.insert(r_id, w_id);
  }
}

#[cfg(debug_assertions)]
mod debug {
  use super::*;
  use std::fmt::{Debug, Formatter, Result};
  impl Debug for WidgetNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
      match self.widget {
        WidgetInstance::Render(ref w) => f.write_str(&w.to_str()),
        WidgetInstance::Combination(ref w) => f.write_str(&w.to_str()),
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[derive(Clone)]
  struct EmbedPost {
    title: &'static str,
    author: &'static str,
    content: &'static str,
    level: usize,
  }

  impl From<EmbedPost> for Widget {
    fn from(c: EmbedPost) -> Self { Widget::Combination(Box::new(c)) }
  }

  struct Text(&'static str);

  impl RenderObject for Text {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { format!("RO::Text({})", self.0) }
    fn paint(&self) {}
    fn perform_layout(&mut self, _ctx: RenderCtx) {}
  }

  impl From<Text> for Widget {
    fn from(t: Text) -> Self { Widget::Render(Box::new(t)) }
  }

  impl<'a> RenderWidget<'a> for Text {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { format!("text({})", self.0) }
    fn create_render_object(&self) -> Box<dyn RenderObject> {
      Box::new(Text(self.0))
    }
  }

  struct RowRenderObject {}

  impl RenderObject for RowRenderObject {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { "RO::Row".to_owned() }
    fn paint(&self) {}
    fn perform_layout(&mut self, _ctx: RenderCtx) {}
  }
  struct RenderRow {}

  impl<'a> RenderWidget<'a> for RenderRow {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { "Render Row".to_owned() }
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

  impl<'a> MultiChildWidget<'a> for Row {
    fn split(
      self: Box<Self>,
    ) -> (Box<dyn for<'r> RenderWidget<'r>>, Vec<Widget>) {
      (Box::new(RenderRow {}), self.children)
    }
  }

  impl<'a> CombinationWidget<'a> for EmbedPost {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { "Embed Post".to_owned() }

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

  #[test]
  #[cfg(debug_assertions)]
  fn widget_tree_inflate() {
    let post = EmbedPost {
      title: "Simple demo",
      author: "Adoo",
      content: "Recursive 3 times",
      level: 3,
    };

    let mut app = Application::new();
    app.inflate(post.into());
    let mut w_tree = String::new();
    let _r = app.widget_tree.write_formatted(&mut w_tree);
    assert_eq!(
      w_tree,
      "Embed Post
└── Render Row
    ├── text(Simple demo)
    ├── text(Adoo)
    ├── text(Recursive 3 times)
    └── Embed Post
        └── Render Row
            ├── text(Simple demo)
            ├── text(Adoo)
            ├── text(Recursive 3 times)
            └── Embed Post
                └── Render Row
                    ├── text(Simple demo)
                    ├── text(Adoo)
                    ├── text(Recursive 3 times)
                    └── Embed Post
                        └── Render Row
                            ├── text(Simple demo)
                            ├── text(Adoo)
                            └── text(Recursive 3 times)
"
    );

    app.construct_render_tree();
    let mut r_tree = String::new();
    let _r = app.render_tree.write_formatted(&mut r_tree);
    assert_eq!(
      r_tree,
      "RO::Row
├── RO::Text(Simple demo)
├── RO::Text(Adoo)
├── RO::Text(Recursive 3 times)
└── RO::Row
    ├── RO::Text(Simple demo)
    ├── RO::Text(Adoo)
    ├── RO::Text(Recursive 3 times)
    └── RO::Row
        ├── RO::Text(Simple demo)
        ├── RO::Text(Adoo)
        ├── RO::Text(Recursive 3 times)
        └── RO::Row
            ├── RO::Text(Simple demo)
            ├── RO::Text(Adoo)
            └── RO::Text(Recursive 3 times)
"
    );
  }

  extern crate test;
  use test::Bencher;

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
      app.construct_render_tree();
    });
  }
}
