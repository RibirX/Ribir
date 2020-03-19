use crate::widget::*;
use ::herald::prelude::*;
use slab_tree::*;
use std::{collections::HashSet, ptr::NonNull};

enum WidgetInstance {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
}

struct WidgetNode {
  w: WidgetInstance,
  subscription: Option<SubscriptionGuard<Box<dyn SubscriptionLike>>>,
}

impl WidgetNode {
  #[inline]
  fn new(w: WidgetInstance) -> Self {
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
            .for_each(|w| stack.push(StackElem::Widget(w)));
          WidgetInstance::Render(render)
        }
      }
    }

    let mut stack = vec![];
    let widget_node = inflate_widget(w, &mut stack);
    let mut node_id = self.build_widget_tree(widget_node);

    loop {
      let elem = stack.pop().unwrap();
      match elem {
        StackElem::NodeID(id) => node_id = id,
        StackElem::Widget(widget) => {
          stack.push(StackElem::NodeID(node_id));

          let widget_node = inflate_widget(widget, &mut stack);
          let new_id = self.preend_widget_by_id(node_id, widget_node);

          stack.push(StackElem::NodeID(new_id));
          self.track_widget_rebuild(new_id);
        }
      }
      if stack.is_empty() {
        break;
      }
    }
  }

  fn preend_widget_by_id(&mut self, id: NodeId, w: WidgetInstance) -> NodeId {
    let mut node = self
      .widget_tree
      .as_mut()
      .expect("root have to exist in logic")
      .get_mut(id)
      .expect("node have to exist in logic");

    node.prepend(WidgetNode::new(w)).node_id()
  }

  fn build_widget_tree(&mut self, w: WidgetInstance) -> NodeId {
    self.widget_tree =
      Some(TreeBuilder::new().with_root(WidgetNode::new(w)).build());
    self.widget_tree.as_mut().unwrap().root_id().unwrap()
  }

  #[inline]
  fn track_widget_rebuild(&mut self, id: NodeId) {
    let w = self.widget_tree.as_mut().unwrap();
    let mut w = w.get_mut(id).unwrap();
    let node = w.data();
    debug_assert!(node.subscription.is_none());
    let mut node_ptr: NonNull<_> = (&mut self.dirty_nodes).into();

    node.subscription = node.w.emitter(self.notifier.clone()).map(|e| {
      // framework logic promise the `node_ptr` always valid.
      e.subscribe(move |_| unsafe {
        node_ptr.as_mut().insert(id);
      })
      .unsubscribe_when_dropped()
    });
  }
}

use std::fmt::{Debug, Formatter, Result};
impl Debug for WidgetNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self.w {
      WidgetInstance::Render(ref w) => f.write_str(&w.to_str()),
      WidgetInstance::Combination(ref w) => f.write_str(&w.to_str()),
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

  impl<'a> RenderWidget<'a> for Text {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { format!("text({})", self.0) }
    fn create_render_object(&self) -> Box<dyn RenderObject> {
      unimplemented!();
    }
  }

  struct RenderRow {}

  impl<'a> RenderWidget<'a> for RenderRow {
    #[cfg(debug_assertions)]
    fn to_str(&self) -> String { "Render Row".to_owned() }
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
  }
}
