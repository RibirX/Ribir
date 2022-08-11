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
  root: WidgetId,
  ctx: Rc<RefCell<AppContext>>,
  pub(crate) state_changed: Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>,
  /// Store the render object's place relative to parent coordinate and the
  /// clamp passed from parent.
  layout_store: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
  pub(crate) generator_store: generator_store::GeneratorStore,
  pub(crate) animations_store: Rc<RefCell<AnimateStore>>,
}

impl WidgetTree {
  pub(crate) fn root(&self) -> WidgetId { self.root }

  pub(crate) fn new(root_widget: Widget, ctx: Rc<RefCell<AppContext>>) -> WidgetTree {
    let mut arena = Arena::default();
    let node: Box<dyn Render> = Box::new(Void);
    let root = WidgetId(arena.new_node(node));
    let ticker = ctx.borrow().frame_ticker.frame_tick_stream();
    let animations_store = Rc::new(RefCell::new(AnimateStore::new(ticker)));
    let mut tree = WidgetTree {
      arena,
      root,
      state_changed: <_>::default(),
      ctx,
      layout_store: <_>::default(),
      generator_store: <_>::default(),
      animations_store,
    };

    let tmp_root = tree.root();
    tmp_root.append_widget(root_widget, &mut tree);
    let real_root = tmp_root.single_child(&tree).unwrap();

    real_root.detach(&mut tree);
    tree.root = real_root;
    tmp_root.remove_subtree(&mut tree);
    tree.mark_dirty(real_root);
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
    let needs_regen = self.generator_store.take_needs_regen_generator();

    if let Some(mut needs_regen) = needs_regen {
      needs_regen.sort_by_cached_key(|g| g.info.parent().ancestors(self).count());
      needs_regen.iter_mut().for_each(|g| {
        if !g.info.parent().is_dropped(self) {
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

  pub(crate) fn place_holder(&mut self) -> WidgetId { self.new_node(Box::new(Void)) }

  pub(crate) fn new_node(&mut self, widget: Box<dyn Render>) -> WidgetId {
    let id = WidgetId(self.arena.new_node(widget));

    id.assert_get(self).query_all_type(
      |notifier: &StateChangeNotifier| {
        let state_changed = self.state_changed.clone();
        notifier
          .change_stream()
          .filter(|b| b.contains(ChangeScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(id);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    id
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.state_changed.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool {
    self.any_state_modified() || self.generator_store.is_dirty()
  }

  pub(crate) fn any_state_modified(&self) -> bool { !self.state_changed.borrow().is_empty() }

  pub(crate) fn any_struct_dirty(&self) -> bool { self.generator_store.is_dirty() }

  pub(crate) fn count(&self) -> usize { self.root.descendants(&self).count() }

  pub(crate) fn context(&self) -> &Rc<RefCell<AppContext>> { &self.ctx }
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
        let split_tree = unsafe { &*(tree as *const WidgetTree) };
        let mut ctx = LayoutCtx { id: self, tree };
        let layout = self.assert_get(split_tree);
        let size = layout.perform_layout(out_clamp, &mut ctx);
        let size = out_clamp.clamp(size);
        let info = tree.layout_info_or_default(self);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;
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

  /// directly remove the widget and not clear any other information.
  pub(crate) fn inner_remove(self, tree: &mut WidgetTree) { self.0.remove(&mut tree.arena) }

  pub(crate) fn remove_subtree(self, tree: &mut WidgetTree) {
    let mut changed = tree.state_changed.borrow_mut();
    self
      .0
      .descendants(&tree.arena)
      .map(WidgetId)
      .for_each(|id| {
        changed.remove(&id);
        tree.generator_store.on_widget_drop(id);
        tree.layout_store.remove(&id);
      });
    self.0.remove_subtree(&mut tree.arena);
  }

  pub(crate) fn detach(self, tree: &mut WidgetTree) { self.0.detach(&mut tree.arena); }

  pub(crate) fn insert_next(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn append_widget(self, widget: Widget, tree: &mut WidgetTree) -> WidgetId {
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
        tree,
      );
    }
    self.last_child(tree).unwrap()
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

  pub(crate) fn insert_child(
    self,
    widget: Widget,
    insert: &mut impl FnMut(Box<dyn Render>, &mut WidgetTree) -> WidgetId,
    consume_child: &mut impl FnMut(WidgetId, Widget, &mut WidgetTree),
    tree: &mut WidgetTree,
  ) -> WidgetId {
    match widget.0 {
      WidgetInner::Compose(c) => {
        let mut build_ctx = BuildCtx::new(Some(self), tree);
        let c = c(&mut build_ctx);
        self.insert_child(c, insert, consume_child, tree)
      }
      WidgetInner::Render(rw) => insert(rw, tree),
      WidgetInner::SingleChild(s) => {
        let (rw, child) = s.unzip();
        let id = insert(rw, tree);
        if let Some(child) = child {
          consume_child(id, child, tree);
        }
        id
      }
      WidgetInner::MultiChild(m) => {
        let (rw, children) = m.unzip();
        let id = insert(rw, tree);
        children
          .into_iter()
          .rev()
          .for_each(|child| consume_child(id, child, tree));
        id
      }
      WidgetInner::Expr(mut e) => {
        let mut ids = smallvec![];
        (e.expr)(&mut |w| {
          let id = self.insert_child(w, insert, consume_child, tree);
          ids.push(id);
        });

        // expr widget, generate at least one widget to anchor itself place.
        if ids.len() == 0 {
          ids.push(self.insert_child(Void.into_widget(), insert, consume_child, tree));
        }
        let last = ids.last().cloned().unwrap();
        tree.generator_store.new_generator(e, self, ids);
        last
      }
    }
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
