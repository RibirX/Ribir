use crate::prelude::*;
use indextree::*;
use smallvec::smallvec;
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

pub(crate) mod animation_store;
mod generator_store;
mod layout_info;
use animation_store::AnimateStore;
pub use layout_info::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
pub(crate) struct WidgetTree {
  arena: Arena<Box<dyn Render>>,
  root: Option<WidgetId>,
  ctx: Rc<RefCell<AppContext>>,
  pub(crate) state_changed: Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>,
  /// Store the render object's place relative to parent coordinate and the
  /// clamp passed from parent.
  layout_store: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
  pub(crate) generator_store: generator_store::GeneratorStore,
  pub(crate) animations_store: Rc<RefCell<AnimateStore>>,
}

impl WidgetTree {
  pub(crate) fn root(&self) -> WidgetId { self.root.expect("Empty tree.") }

  pub(crate) fn new_node(&mut self, node: Box<dyn Render>) -> WidgetId {
    WidgetId(self.arena.new_node(node))
  }

  pub(crate) fn new(root_widget: Widget, ctx: Rc<RefCell<AppContext>>) -> WidgetTree {
    let ticker = ctx.borrow().frame_ticker.frame_tick_stream();
    let animations_store = Rc::new(RefCell::new(AnimateStore::new(ticker)));
    let mut tree = WidgetTree {
      arena: Arena::default(),
      root: None,
      state_changed: <_>::default(),
      ctx,
      layout_store: <_>::default(),
      generator_store: <_>::default(),
      animations_store,
    };

    tree.insert_widget_to(None, root_widget, |node, tree| {
      let root = tree.new_node(node);
      tree.set_root(root);
      root.on_mounted(tree);
      root
    });
    tree.mark_dirty(tree.root());
    tree
  }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self, painter: &mut Painter) {
    let mut w = Some(self.root());

    let mut paint_ctx = PaintingCtx::new(self.root(), self, painter);
    while let Some(id) = w {
      paint_ctx.id = id;
      let rect = paint_ctx
        .box_rect()
        .expect("when paint node, it's mut be already layout.");
      paint_ctx
        .painter
        .save()
        .translate(rect.min_x(), rect.min_y());
      let rw = id.assert_get(self);
      rw.paint(&mut paint_ctx);

      w = id
        // deep first.
        .first_child(self)
        // goto sibling or back to parent sibling
        .or_else(|| {
          let mut node = w;
          while let Some(p) = node {
            // self node sub-tree paint finished, goto sibling
            paint_ctx.painter.restore();
            node = p.next_sibling(self);
            if node.is_some() {
              break;
            } else {
              // if there is no more sibling, back to parent to find sibling.
              node = p.parent(self);
            }
          }
          node
        });
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed.
  pub(crate) fn tree_repair(&mut self) {
    while let Some(mut needs_regen) = self.generator_store.take_needs_regen_generator() {
      needs_regen
        .sort_by_cached_key(|g| g.info.parent().map_or(0, |wid| wid.ancestors(self).count()));
      needs_regen.iter_mut().for_each(|g| {
        if !g.info.parent().map_or(false, |p| p.is_dropped(self)) {
          g.update_generated_widgets(self);
        }
      });

      needs_regen
        .into_iter()
        .for_each(|g| self.generator_store.add_generator(g));
    }
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) -> bool {
    let mut performed_layout = false;
    loop {
      if let Some(needs_layout) = self.layout_list() {
        performed_layout = performed_layout || !needs_layout.is_empty();
        needs_layout.iter().for_each(|wid| {
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          wid.perform_layout(clamp, self);
        });
      } else {
        break;
      }
    }
    performed_layout
  }

  pub(crate) fn set_root(&mut self, root: WidgetId) {
    assert!(self.root.is_none());
    self.root = Some(root);
  }

  pub(crate) fn insert_widget_to(
    &mut self,
    parent: Option<WidgetId>,
    widget: Widget,
    mount_node: impl FnMut(Box<dyn Render>, &mut WidgetTree) -> WidgetId,
  ) -> WidgetId {
    let mut stack = vec![];
    let id = self.widget_to_node(widget, parent, mount_node, &mut stack);

    while let Some((widget, parent)) = stack.pop() {
      self.widget_to_node(
        widget,
        Some(parent),
        |c, tree| {
          let child = tree.new_node(c);
          parent.append(child, tree);
          child.on_mounted(tree);
          child
        },
        &mut stack,
      );
    }

    id
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.state_changed.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool {
    self.any_state_modified() || self.generator_store.is_dirty()
  }

  pub(crate) fn any_state_modified(&self) -> bool { !self.state_changed.borrow().is_empty() }

  pub(crate) fn any_struct_dirty(&self) -> bool { self.generator_store.is_dirty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self).count() }

  pub(crate) fn context(&self) -> &Rc<RefCell<AppContext>> { &self.ctx }

  pub(crate) unsafe fn split_tree(&mut self) -> (&mut WidgetTree, &mut WidgetTree) {
    let ptr = self as *mut WidgetTree;
    (&mut *ptr, &mut *ptr)
  }

  fn widget_to_node(
    &mut self,
    widget: Widget,
    parent: Option<WidgetId>,
    mut on_node: impl FnMut(Box<dyn Render>, &mut WidgetTree) -> WidgetId,
    stack: &mut Vec<(Widget, WidgetId)>,
  ) -> WidgetId {
    match widget.0 {
      WidgetInner::Compose(c) => {
        let mut build_ctx = BuildCtx::new(parent, self);
        let c = c(&mut build_ctx);
        self.widget_to_node(c, parent, on_node, stack)
      }
      WidgetInner::Render(rw) => on_node(rw, self),
      WidgetInner::SingleChild(s) => {
        let (rw, child) = s.unzip();
        let id = on_node(rw, self);
        if let Some(child) = child {
          stack.push((child, id));
        }
        id
      }
      WidgetInner::MultiChild(m) => {
        let (rw, children) = m.unzip();
        let p = on_node(rw, self);
        children
          .into_iter()
          .rev()
          .for_each(|child| stack.push((child, p)));
        p
      }
      WidgetInner::Expr(e) => {
        let road_sign = on_node(Box::new(Void), self);
        self
          .generator_store
          .new_generator(e, parent, smallvec![road_sign]);
        road_sign
      }
    }
  }
}

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &WidgetTree) -> Option<&dyn Render> {
    tree.arena.get(self.0).map(|node| node.get().as_ref())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut Box<dyn Render>> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// Compute layout of the render widget `id`, and store its result in the
  /// store.
  pub(crate) fn perform_layout(self, out_clamp: BoxClamp, tree: &mut WidgetTree) -> Size {
    tree
      .layout_info(self)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        // Safety: `LayoutCtx` will never mutable access widget tree, so split a node is
        // safe.
        let (tree1, tree2) = unsafe { tree.split_tree() };
        let mut ctx = LayoutCtx { id: self, tree: tree1 };
        let layout = self.assert_get(tree2);
        let size = layout.perform_layout(out_clamp, &mut ctx);
        let size = out_clamp.clamp(size);
        let info = tree1.layout_info_or_default(self);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;

        self.assert_get_mut(tree1).query_all_type_mut(
          |l: &mut PerformedLayoutListener| {
            (l.on_performed_layout)(LifeCycleCtx { id: self, tree: tree2 });
            true
          },
          QueryOrder::OutsideFirst,
        );
        size
      })
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
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    let (tree1, tree3) = unsafe { tree1.split_tree() };
    let mut changed = tree1.state_changed.borrow_mut();
    self
      .0
      .descendants(&tree1.arena)
      .map(WidgetId)
      .for_each(|id| {
        changed.remove(&id);
        tree1.generator_store.on_widget_drop(id);
        tree1.layout_store.remove(&id);
        id.assert_get_mut(tree2).query_all_type_mut(
          |d: &mut DisposedListener| {
            (d.on_disposed)(LifeCycleCtx { id, tree: tree3 });
            true
          },
          QueryOrder::OutsideFirst,
        )
      });
    self.0.remove_subtree(&mut tree1.arena);
    if tree1.root() == self {
      tree1.root.take();
    }
  }

  pub(crate) fn detach(self, tree: &mut WidgetTree) {
    if Some(self) == tree.root {
      tree.root.take();
    }
    self.0.detach(&mut tree.arena);
  }

  pub(crate) fn insert_before(self, before: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_before(before.0, &mut tree.arena);
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn on_mounted(self, tree: &mut WidgetTree) {
    self.assert_get(tree).query_all_type(
      |notifier: &StateChangeNotifier| {
        let state_changed = tree.state_changed.clone();
        notifier
          .change_stream()
          .filter(|b| b.contains(ChangeScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(self);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    // Safety: lifecycle context have no way to change tree struct.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get_mut(tree1).query_all_type_mut(
      |m: &mut MountedListener| {
        (m.on_mounted)(LifeCycleCtx { id: self, tree: tree2 });
        true
      },
      QueryOrder::OutsideFirst,
    );
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &WidgetTree) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree));
    self.first_child(tree)
  }

  fn node_feature<F: Fn(&Node<Box<dyn Render>>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub(crate) fn assert_get(self, tree: &WidgetTree) -> &dyn Render {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut Box<dyn Render> {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use std::{cell::RefCell, rc::Rc};

  use test::Bencher;

  use super::*;
  use crate::{
    prelude::{widget_tree::WidgetTree, IntoWidget},
    test::{embed_post::EmbedPost, key_embed_post::EmbedPostWithKey, recursive_row::RecursiveRow},
  };

  fn test_sample_create(width: usize, depth: usize) -> WidgetTree {
    WidgetTree::new(RecursiveRow { width, depth }.into_widget(), <_>::default())
  }

  #[test]
  fn drop_info_clear() {
    let post = EmbedPost::new(3);
    let ctx = Rc::new(RefCell::new(AppContext::default()));
    let mut tree = WidgetTree::new(post.into_widget(), ctx);
    tree.tree_repair();
    assert_eq!(tree.count(), 17);

    tree.mark_dirty(tree.root());
    tree.root().remove_subtree(&mut tree);

    assert_eq!(tree.is_dirty(), false);
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost::new(1000);
      WidgetTree::new(post.into_widget(), <_>::default());
    });
  }

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
    let post = EmbedPostWithKey::new(1000);
    let mut tree = WidgetTree::new(post.into_widget(), <_>::default());
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair()
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let mut tree = test_sample_create(50, 2);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let mut tree = test_sample_create(100, 2);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let mut tree = test_sample_create(10, 4);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let mut tree = test_sample_create(10, 5);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }
}
