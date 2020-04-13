use crate::{prelude::*, util::TreeFormatter};

use indextree::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);

#[derive(Default)]
pub struct WidgetTree<'a> {
  arena: Arena<Widget<'a>>,
  root: Option<WidgetId>,
}

impl<'a> WidgetTree<'a> {
  #[inline]
  pub fn root(&self) -> Option<WidgetId> { self.root }

  #[inline]
  pub fn set_root(&mut self, data: Widget<'a>) -> WidgetId {
    debug_assert!(self.root.is_none());
    let root = self.new_node(data);
    self.root = Some(root);
    root
  }

  #[inline]
  pub fn new_node(&mut self, data: Widget<'a>) -> WidgetId {
    WidgetId(self.arena.new_node(data))
  }
  
  /// inflate  subtree, so every subtree leaf should be a Widget::Render.
  pub(crate) fn inflate(&mut self, wid: WidgetId) -> &mut Self {
    let mut stack = vec![wid];

    fn append<'a>(
      parent: WidgetId,
      widget: Widget<'a>,
      stack: &mut Vec<WidgetId>,
      tree: &mut WidgetTree<'a>,
    ) {
      let node = parent.append_widget(widget, tree);
      stack.push(node);
    }

    while let Some(parent) = stack.pop() {
      let p_widget = parent.get_mut(self).expect("must exist!");
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
    self
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
  /// Returns a reference to the node data.
  pub fn get<'a>(self, tree: &'a WidgetTree) -> Option<&'a Widget<'a>> {
    tree.arena.get(self.0).map(|node| node.get())
  }

  /// Returns a mutable reference to the node data.
  pub fn get_mut<'a, 'b>(
    self,
    tree: &'b mut WidgetTree<'a>,
  ) -> Option<&'b mut Widget<'a>> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  pub fn append_widget<'a>(
    self,
    data: Widget<'a>,
    tree: &mut WidgetTree<'a>,
  ) -> WidgetId {
    let child = tree.new_node(data);
    self.append(child, tree);
    child
  }

  /// A delegate for [NodeId::append](indextree::NodeId.append)
  pub fn append(self, new_child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  pub fn remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena); }

  /// A delegate for [NodeId::parent](indextree::NodeId.parent)
  pub fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.parent())
  }

  /// A delegate for [NodeId::first_child](indextree::NodeId.first_child)
  pub fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.first_child())
  }

  /// A delegate for [NodeId::last_child](indextree::NodeId.last_child)
  pub fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.last_child())
  }

  /// A delegate for
  /// [NodeId::previous_sibling](indextree::NodeId.previous_sibling)
  pub fn previous_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.previous_sibling())
  }

  /// A delegate for [NodeId::next_sibling](indextree::NodeId.next_sibling)
  pub fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_id_feature(tree, |node| node.next_sibling())
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub fn ancestors<'a>(
    self,
    tree: &'a WidgetTree,
  ) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.ancestors(&tree.arena).map(|id| WidgetId(id))
  }

  /// A delegate for [NodeId::descendants](indextree::NodeId.descendants)
  pub fn descendants<'a>(
    self,
    tree: &'a WidgetTree,
  ) -> impl Iterator<Item = WidgetId> + 'a {
    self.0.descendants(&tree.arena).map(|id| WidgetId(id))
  }

  /// A delegate for [NodeId::detach](indextree::NodeId.detach)
  pub fn detach(&self, tree: &mut WidgetTree) {
    self.0.detach(&mut tree.arena);
    if tree.root == Some(*self) {
      tree.root = None;
    }
  }

  /// create a render object from this widget, return the created render object and 
  /// the widget id which actually created the render object.
  pub(crate) fn create_render_object(
    &self,
    tree: &WidgetTree,
  ) -> ( Box<dyn RenderObjectSafety + Send + Sync>, WidgetId) {
    let id = self.down_nearest_render_widget(tree);
   let render_obj = match id.get(&tree).expect("must exists!") {
      Widget::Combination(_) => {
        unreachable!("only render widget can create render object!")
      }
      Widget::Render(r) => r.create_render_object(),
      Widget::SingleChild(r) => r.create_render_object(),
      Widget::MultiChild(r) => r.create_render_object(),
    };
    (render_obj, id)
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
  pub(crate) fn down_nearest_render_widget(
    self,
    tree: &WidgetTree,
  ) -> WidgetId {
    let mut wid = self;
    while let Some(Widget::Combination(_)) = wid.get(tree) {
      wid = wid.single_child(tree);
    }
    debug_assert!(!matches!(&wid.get(&tree).unwrap(), Widget::Combination(_)));
    wid
  }

  /// find the nearest render widget in ancestors, include self.
  pub(crate) fn upper_nearest_render_widget(
    self,
    tree: &WidgetTree,
  ) -> WidgetId {
    let wid = self
      .ancestors(tree)
      .find(|id| !matches!(id.get(tree), Some(Widget::Combination(_))))
      .expect(
        "should only call this method if `wid`  have render widget ancestor!",
      );

    debug_assert!(matches!(wid.get(tree).unwrap(), Widget::Render(_)));

    wid
  }

  fn node_id_feature<F: Fn(&Node<Widget>) -> Option<NodeId>>(
    &self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree
      .arena
      .get(self.0)
      .map(method)
      .flatten()
      .map(|id| WidgetId(id))
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::test::embed_post::EmbedPost;
  extern crate test;
  use test::Bencher;

  fn create_env<'a>(level: usize) -> (WidgetTree<'a>, WidgetId) {
    let mut tree = WidgetTree::default();
    let root = tree.set_root(EmbedPost::new(level).to_widget());
    (tree, root)
  }

  #[test]
  fn infate_tree() {
    let (mut tree, root) = create_env(3);
    tree.inflate(root);
    assert_eq!(
      tree.symbol_shape(),
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
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let (mut tree, root) = create_env(1000);
      tree.inflate(root);
    });
  }
}
