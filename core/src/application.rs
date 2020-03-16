use crate::widget::Widget;
use ::herald::prelude::*;
use slab_tree::*;
use std::{collections::HashSet, ptr::NonNull};

struct WidgetNode {
  w: Widget,
  subscription: Option<SubscriptionGuard<Box<dyn SubscriptionLike>>>,
}

impl WidgetNode {
  #[inline]
  fn new(w: Widget) -> Self {
    WidgetNode {
      w,
      subscription: None,
    }
  }
}

#[derive(Default)]
pub struct Application<'a> {
  notifier: LocalSubject<'a, (), ()>,
  widget_tree: Option<Tree<WidgetNode>>,
  dirty_nodes: HashSet<NodeId>,
}

impl<'a> Application<'a> {
  pub fn new() -> Application<'a> {
    Application {
      widget_tree: None,
      ..Default::default()
    }
  }

  pub fn run(mut self, w: Widget) {
    self.inflate(w);
    todo!("implement render tree");
  }

  fn inflate(&mut self, w: Widget) {
    enum StackElem {
      Widget(Widget),
      NodeID(NodeId),
    }

    /// Return an widget after inflated, and store the sub widgets into the
    /// `stack`
    #[inline]
    fn inflate_widget(widget: Widget, stack: &mut Vec<StackElem>) -> Widget {
      match widget {
        Widget::Combination(w) => {
          let c = w.build();
          stack.push(StackElem::Widget(c));
          Widget::Combination(w)
        }
        w @ Widget::Render(_) => w,
        Widget::SingleChild(w) => {
          let (render, child) = w.split();
          stack.push(StackElem::Widget(child));
          Widget::Render(render)
        }
        Widget::MultiChild(w) => {
          let (render, children) = w.split();
          children
            .into_iter()
            .for_each(|w| stack.push(StackElem::Widget(w)));
          Widget::Render(render)
        }
      }
    }

    let mut stack = vec![StackElem::Widget(w)];
    let mut node_id: Option<NodeId> = None;

    loop {
      let elem = stack.pop().unwrap();
      match elem {
        StackElem::NodeID(id) => node_id = Some(id),
        StackElem::Widget(widget) => {
          if let Some(id) = node_id {
            stack.push(StackElem::NodeID(id));
          }
          let widget_node = inflate_widget(widget, &mut stack);
          let new_id = self.add_widget(node_id, widget_node);
          stack.push(StackElem::NodeID(new_id));
        }
      }
      if stack.is_empty() {
        break;
      }
    }
  }

  /// If `id` is `Some`-value add a widget into widget tree as a child of node
  /// which node_id is `id`, if `id` is `None`-value, use `w` as root widget.
  /// Return new node's node_id.
  #[inline]
  fn add_widget(&mut self, id: Option<NodeId>, w: Widget) -> NodeId {
    let id = if let Some(id) = id {
      let mut node = self
        .widget_tree
        .as_mut()
        .expect("root have to exist in logic")
        .get_mut(id)
        .expect("node have to exist in logic");

      node.prepend(WidgetNode::new(w)).node_id()
    } else {
      let tree = TreeBuilder::new().with_root(WidgetNode::new(w)).build();
      let root_id = tree.root_id().expect("assert root");
      self.widget_tree = Some(tree);
      root_id
    };
    self.track_widget_rebuild(id);
    id
  }

  #[inline]
  fn track_widget_rebuild(&mut self, id: NodeId) {
    let w = self.widget_tree.as_mut().unwrap();
    let mut w = w.get_mut(id).unwrap();
    let node = w.data();
    assert!(node.subscription.is_none());
    let mut node_ptr: NonNull<_> = (&mut self.dirty_nodes).into();
    if let Widget::Combination(ref mut w) = node.w {
      node.subscription = w.emitter(self.notifier.clone()).map(|e| {
        // framework logic promise the `node_ptr` always valid.
        e.subscribe(move |_| unsafe {
          node_ptr.as_mut().insert(id);
        })
        .unsubscribe_when_dropped()
      });
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::prelude::*;

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

  impl From<Text> for Widget {
    fn from(t: Text) -> Self { Widget::Render(Box::new(t)) }
  }

  impl RenderWidget for Text {
    fn create_render_object(&self) -> Box<dyn RenderObject> {
      unimplemented!();
    }
  }

  struct RenderRow {}

  impl RenderWidget for RenderRow {
    fn create_render_object(&self) -> Box<dyn RenderObject> {
      unimplemented!();
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

  impl MultiChildWidget for Row {
    fn split(self: Box<Self>) -> (Box<dyn RenderWidget>, Vec<Widget>) {
      (Box::new(RenderRow {}), self.children)
    }
  }

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

  use std::fmt::{Debug, Formatter, Result};
  impl Debug for WidgetNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
      match self.w {
        Widget::SingleChild(_) => f.write_str("single-child"),
        Widget::MultiChild(_) => f.write_str("multi-child"),
        Widget::Render(_) => f.write_str("render"),
        Widget::Combination(_) => f.write_str("combination"),
      }
    }
  }

  #[test]
  fn widget_tree_inflate() {
    let post = EmbedPost {
      title: "Simple demo",
      author: "Adoo",
      content: "Recursive 3 times",
      level: 3,
    };

    let mut app = Application::new();
    app.inflate(post.into());
    let mut fmt_tree = String::new();
    let _r = app.widget_tree.unwrap().write_formatted(&mut fmt_tree);
    assert_eq!(
      fmt_tree,
      "combination
└── render
    ├── render
    ├── render
    ├── render
    └── combination
        └── render
            ├── render
            ├── render
            ├── render
            └── combination
                └── render
                    ├── render
                    ├── render
                    ├── render
                    └── combination
                        └── render
                            ├── render
                            ├── render
                            └── render
"
    );
  }
}
