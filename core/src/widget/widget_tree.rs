use crate::prelude::*;
use indextree::*;
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

mod layout_info;
pub use layout_info::*;

use super::Children;

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
  pub(crate) needs_regen: Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>,
}

impl WidgetTree {
  pub(crate) fn root(&self) -> WidgetId { self.root.expect("Empty tree.") }

  pub(crate) fn new_node(&mut self, node: Box<dyn Render>) -> WidgetId {
    WidgetId(self.arena.new_node(node))
  }

  pub(crate) fn empty_node(&mut self) -> WidgetId { self.new_node(Box::new(Void)) }

  pub(crate) fn new(root_widget: Widget, ctx: Rc<RefCell<AppContext>>) -> WidgetTree {
    let mut tree = WidgetTree {
      arena: Arena::default(),
      root: None,
      state_changed: <_>::default(),
      ctx,
      layout_store: <_>::default(),
      needs_regen: <_>::default(),
    };

    tree.set_root(root_widget);
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

  // perform repair the tree and layout it until everything ready, and return if
  // modify the tree struct.
  pub(crate) fn tree_ready(&mut self, win_size: Size) -> bool {
    let mut struct_modify = false;
    while self.is_dirty() {
      struct_modify |= self.any_struct_dirty();
      self.tree_repair();
      self.layout(win_size);
    }
    struct_modify
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed.
  pub(crate) fn tree_repair(&mut self) {
    loop {
      if self.needs_regen.borrow().is_empty() {
        break;
      }

      let mut needs_regen = self
        .needs_regen
        .borrow_mut()
        .drain()
        .filter(|g| !g.0.is_removed(&mut self.arena))
        .collect::<Vec<_>>();

      needs_regen.sort_by_cached_key(|g| g.ancestors(self).count());
      for g in needs_regen.into_iter() {
        // child expr widget may removed by parent ancestor expr widget refresh.
        if !g.is_dropped(self) {
          self.refresh_generator(g);
        }
      }
    }
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    let mut performed = vec![];

    loop {
      if let Some(needs_layout) = self.layout_list() {
        needs_layout.iter().for_each(|wid| {
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          wid.perform_layout(clamp, self, &mut performed);
        });
      } else {
        break;
      }
    }

    performed.drain(..).for_each(|id| {
      let (tree1, tree2) = unsafe { self.split_tree() };
      id.assert_get(tree1).query_all_type(
        |l: &PerformedLayoutListener| {
          (l.on_performed_layout.borrow_mut())(LifeCycleCtx { id, tree: tree2 });
          true
        },
        QueryOrder::OutsideFirst,
      );
    });
  }

  pub(crate) fn set_root(&mut self, widget: Widget) {
    assert!(self.root.is_none());

    let root = widget.into_subtree(None, self).expect("must have a root");
    self.set_root_id(root);
    root.on_mounted_subtree(self, true);
    self.mark_dirty(root);
  }

  pub(crate) fn set_root_id(&mut self, id: WidgetId) {
    assert!(self.root.is_none());
    self.root = Some(id);
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.state_changed.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { self.any_state_modified() || self.any_struct_dirty() }

  pub(crate) fn any_state_modified(&self) -> bool { !self.state_changed.borrow().is_empty() }

  pub(crate) fn any_struct_dirty(&self) -> bool { !self.needs_regen.borrow().is_empty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self).count() }

  pub(crate) fn app_ctx(&self) -> &Rc<RefCell<AppContext>> { &self.ctx }

  pub(crate) unsafe fn split_tree(&mut self) -> (&mut WidgetTree, &mut WidgetTree) {
    let ptr = self as *mut WidgetTree;
    (&mut *ptr, &mut *ptr)
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
  pub(crate) fn perform_layout(
    self,
    out_clamp: BoxClamp,
    tree: &mut WidgetTree,
    performed: &mut Vec<WidgetId>,
  ) -> Size {
    tree
      .layout_info(self)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        // Safety: `LayoutCtx` will never mutable access widget tree, so split a node is
        // safe.
        let (tree1, tree2) = unsafe { tree.split_tree() };
        let mut ctx = LayoutCtx { id: self, tree: tree1, performed };
        let layout = self.assert_get(tree2);
        let size = layout.perform_layout(out_clamp, &mut ctx);
        let size = out_clamp.clamp(size);
        let info = tree1.layout_info_or_default(self);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;

        self.assert_get(tree1).query_all_type(
          |_: &PerformedLayoutListener| {
            performed.push(self);
            false
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

  pub(crate) fn replace_data(
    self,
    other: Box<dyn Render>,
    tree: &mut WidgetTree,
  ) -> Box<dyn Render> {
    std::mem::replace(self.assert_get_mut(tree), other)
  }

  pub(crate) fn swap_data(self, other: WidgetId, tree: &mut WidgetTree) {
    // Safety: mut borrow two node not intersect.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    std::mem::swap(self.assert_get_mut(tree1), other.assert_get_mut(tree2));
  }

  pub(crate) fn swap(self, other: WidgetId, tree: &mut WidgetTree) {
    let first_child = self.first_child(tree);
    let mut cursor = first_child;
    while let Some(c) = cursor {
      cursor = c.next_sibling(tree);
      other.append(c, tree);
    }
    let mut other_child = other.first_child(tree);
    while other_child.is_some() && other_child != first_child {
      let o_c = other_child.unwrap();
      other_child = o_c.next_sibling(tree);
      self.append(o_c, tree);
    }

    let guard = tree.empty_node();
    self.insert_after(guard, tree);
    other.insert_after(self, tree);
    guard.insert_after(other, tree);
    guard.0.remove(&mut tree.arena);
  }

  pub(crate) fn remove_subtree(self, tree: &mut WidgetTree) {
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self
      .0
      .descendants(&tree1.arena)
      .map(WidgetId)
      .for_each(|id| id.on_disposed(tree2));
    self.0.remove_subtree(&mut tree1.arena);
    if tree1.root() == self {
      tree1.root.take();
    }
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn insert_before(self, before: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_before(before.0, &mut tree.arena);
  }

  pub(crate) fn prepend(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.prepend(child.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn on_mounted_subtree(self, tree: &mut WidgetTree, brand_new: bool) {
    let (tree1, tree2) = unsafe { tree.split_tree() };

    self
      .descendants(tree1)
      .for_each(|w| w.on_mounted(tree2, brand_new));
  }

  pub(crate) fn on_mounted(self, tree: &mut WidgetTree, brand_new: bool) {
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

    if brand_new {
      // Safety: lifecycle context have no way to change tree struct.
      let (tree1, tree2) = unsafe { tree.split_tree() };
      self.assert_get(tree1).query_all_type(
        |m: &MountedListener| {
          (m.on_mounted.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 });
          true
        },
        QueryOrder::OutsideFirst,
      );
    }
  }

  pub(crate) fn on_disposed(self, tree: &mut WidgetTree) {
    tree.layout_store.remove(&self);
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get(tree1).query_all_type(
      |d: &DisposedListener| {
        (d.on_disposed.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 });
        true
      },
      QueryOrder::OutsideFirst,
    )
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

impl Widget {
  pub(crate) fn into_subtree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
  ) -> Option<WidgetId> {
    let (id, children) = self.place_node_in_tree(parent, tree);
    if let Some(id) = id {
      let mut pairs = vec![];
      children.for_each(|w| pairs.push((id, w)));

      while let Some((parent, widget)) = pairs.pop() {
        let (child, children) = widget.place_node_in_tree(Some(parent), tree);
        if let Some(child) = child {
          parent.prepend(child, tree);
        }
        children.for_each(|w| pairs.push((child.unwrap_or(parent), w)));
      }
      Some(id)
    } else {
      match children {
        Children::None => None,
        _ => unreachable!(),
      }
    }
  }

  fn place_node_in_tree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
  ) -> (Option<WidgetId>, Children) {
    let Self { node, children } = self;

    if let Some(node) = node {
      match node {
        WidgetNode::Compose(c) => {
          assert!(children.is_none(), "compose widget shouldn't have child.");
          let mut build_ctx = BuildCtx::new(parent, tree);
          let c = c(&mut build_ctx);
          c.place_node_in_tree(parent, tree)
        }
        WidgetNode::Render(r) => (Some(tree.new_node(r)), children),
        WidgetNode::Dynamic(e) => {
          let w = Generator::new_generator(e, !children.is_none(), tree);
          (Some(w), children)
        }
      }
    } else {
      match children {
        Children::None => (None, Children::None),
        Children::Single(s) => s.place_node_in_tree(parent, tree),
        Children::Multi(_) => unreachable!("None parent with multi child is forbidden."),
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
  fn fix_dropped_child_expr_widget() {
    let parent = Stateful::new(true);
    let child = Stateful::new(true);
    let w = widget! {
      track { parent: parent.clone(), child: child.clone() }
      ExprWidget {
        expr: parent.then(|| {
          widget!{
            SizedBox {
              size: Size::zero(),
              ExprWidget { expr: child.then(|| Void )}
            }
          }
        })
      }
    };

    let mut wnd = Window::without_render(w, Size::new(100., 100.));
    wnd.draw_frame();

    {
      *child.state_ref() = false;
      *parent.state_ref() = false;
    }

    // fix crash here.
    wnd.draw_frame();
  }

  #[test]
  fn fix_child_expr_widget_same_root_as_parent() {
    let trigger = Stateful::new(true);
    let w = widget! {
      track { trigger: trigger.clone() }
      ExprWidget {
        expr: trigger.then(|| {
          widget!{ ExprWidget { expr: trigger.then(|| Void )}}
        })
      }
    };

    let mut wnd = Window::without_render(w, Size::new(100., 100.));
    wnd.draw_frame();

    {
      *trigger.state_ref() = false;
    }

    // fix crash here
    // crash because generator live as long as its parent, at here two expr widget's
    // parent both none, all as root expr widget, parent expr widget can't remove
    // child expr widget.
    //
    // generator lifetime should bind to its generator widget instead of parent.
    wnd.draw_frame();
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
    assert_eq!(tree.layout_list(), None);
    assert!(
      tree
        .needs_regen
        .borrow()
        .iter()
        .all(|g| g.is_dropped(&tree))
    );
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

  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    let trigger = Stateful::new(1);
    let widget = widget! {
      track { trigger: trigger.clone() }
      Row {
        ExprWidget {
          expr: (0..3).map(|_| if *trigger > 0 {
            SizedBox { size: Size::new(1., 1.)}
          } else {
            SizedBox { size: Size::zero()}
          }).collect::<Vec<_>>()
        }
      }
    };

    let mut tree = WidgetTree::new(widget, <_>::default());
    tree.tree_repair();
    tree.layout(Size::new(100., 100.));
    {
      *trigger.silent_ref() = 2;
    }
    assert!(tree.needs_regen.borrow().is_empty())
  }
}
