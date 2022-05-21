use crate::prelude::*;
use indextree::*;
use smallvec::SmallVec;
use std::pin::Pin;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
pub(crate) struct WidgetTree {
  arena: Arena<Box<dyn RenderNode>>,
  root: WidgetId,
}

impl WidgetTree {
  pub(crate) fn new() -> Pin<Box<Self>> {
    let mut arena = Arena::default();
    let node: Box<dyn RenderNode> = Box::new(Void);
    let root = WidgetId(arena.new_node(node));
    let tree = Self { arena, root };
    let mut tree = Box::pin(tree);
    tree.as_mut().widget_info_assign(root);
    tree
  }

  pub(crate) fn root(&self) -> WidgetId { self.root }

  pub(crate) fn reset_root(&mut self, new_root: WidgetId) -> WidgetId {
    let old = self.root;
    old.detach(self);
    self.root = new_root;
    old
  }

  pub(crate) fn place_holder(&mut self) -> WidgetId { self.new_node(Box::new(Void)) }

  pub(crate) fn new_node(&mut self, widget: Box<dyn RenderNode>) -> WidgetId {
    let id = WidgetId(self.arena.new_node(widget));
    id
  }

  #[cfg(test)]
  pub(crate) fn count(&self) -> usize { self.arena.count() }

  fn widget_info_assign(&mut self, _id: WidgetId) { todo!("stateful subscribe") }
}

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &WidgetTree) -> Option<&dyn RenderNode> {
    tree.arena.get(self.0).map(|node| node.get().as_ref())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut Box<dyn RenderNode>> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &WidgetTree) -> bool { self.0.is_removed(&tree.arena) }

  #[allow(clippy::needless_collect)]
  pub(crate) fn common_ancestor_of(self, other: WidgetId, tree: &WidgetTree) -> Option<WidgetId> {
    if self.is_dropped(tree) || other.is_dropped(tree) {
      return None;
    }

    let p0 = other.ancestors(tree).collect::<Vec<_>>();
    let p1 = self.ancestors(tree).collect::<Vec<_>>();

    p0.iter()
      .rev()
      .zip(p1.iter().rev())
      .filter(|(a, b)| a == b)
      .last()
      .map(|(p, _)| p.clone())
  }

  pub(crate) fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  pub(crate) fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  pub(crate) fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  pub(crate) fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  pub(crate) fn prev_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.previous_sibling())
  }

  pub(crate) fn ancestors(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// Detect if this widget is the ancestors of `w`
  pub(crate) fn ancestors_of(self, w: WidgetId, tree: &WidgetTree) -> bool {
    w.ancestors(tree).any(|a| a == self)
  }

  pub(crate) fn children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn reverse_children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.reverse_children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn descendants(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  pub(crate) fn remove_subtree(self, tree: &mut WidgetTree) {
    self.0.remove_subtree(&mut tree.arena);
  }

  pub(crate) fn detach(self, tree: &mut WidgetTree) { self.0.detach(&mut tree.arena); }

  pub(crate) fn insert_next(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn append_widget(self, widget: Widget, ctx: &mut Context) -> WidgetId {
    let mut stack = vec![(widget, self)];

    while let Some((widget, p_wid)) = stack.pop() {
      p_wid.insert_child(
        widget,
        &mut |node, tree| {
          let wid = tree.new_node(node);
          p_wid.append(wid, tree);
          wid
        },
        &mut |id, child, _| stack.push((child, id)),
        ctx,
      );
    }
    self.last_child(ctx.tree()).unwrap()
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &WidgetTree) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree));
    self.first_child(tree)
  }

  fn node_feature<F: Fn(&Node<Box<dyn RenderNode>>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub(crate) fn assert_get(self, tree: &WidgetTree) -> &dyn RenderNode {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut Box<dyn RenderNode> {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn insert_child(
    self,
    widget: Widget,
    insert: &mut impl FnMut(Box<dyn RenderNode>, &mut WidgetTree) -> WidgetId,
    consume_child: &mut impl FnMut(WidgetId, Widget, &mut Context),
    ctx: &mut Context,
  ) -> WidgetId {
    let tree = ctx.widget_tree.as_mut().get_mut();
    let mut insert_widget = |node, tree: &mut WidgetTree| {
      let id = insert(node, tree);
      tree.widget_info_assign(id);
      id
    };
    match widget.0 {
      WidgetInner::Compose(c) => {
        let mut build_ctx = BuildCtx::new(Some(self), ctx);
        let c = c(&mut build_ctx);
        self.insert_child(c, insert, consume_child, ctx)
      }
      WidgetInner::Render(rw) => insert_widget(rw, tree),
      WidgetInner::SingleChild(s) => {
        let (rw, child) = s.unzip();
        let id = insert_widget(rw, tree);
        if let Some(child) = child {
          consume_child(id, child, ctx);
        }
        id
      }
      WidgetInner::MultiChild(m) => {
        let (rw, children) = m.unzip();
        let id = insert_widget(rw, tree);
        children
          .into_iter()
          .rev()
          .for_each(|child| consume_child(id, child, ctx));
        id
      }
      WidgetInner::Expr(mut e) => {
        let widgets = (e.expr)().collect::<Vec<_>>();
        let mut ids = widgets
          .into_iter()
          .map(|x| self.insert_child(x, insert, consume_child, ctx))
          .collect::<SmallVec<_>>();

        // expr widget, generate at least one widget to anchor itself place.
        if ids.len() == 0 {
          ids.push(self.insert_child(Void.into_widget(), insert, consume_child, ctx));
        }
        let last = ids.last().cloned().unwrap();
        ctx.generator_store.new_generator(e, self, ids);
        last
      }
    }
  }
}
