use indextree::*;
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

mod layout_info;
use crate::prelude::*;
pub use layout_info::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
pub(crate) struct WidgetTree {
  arena: Arena<Box<dyn Render>>,
  root: Option<WidgetId>,
  ctx: AppContext,
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

  pub(crate) fn new(root_widget: Widget, ctx: AppContext) -> WidgetTree {
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
      paint_ctx.painter.save();

      let need_paint: bool = !id.is_offstage(self);
      if need_paint {
        let rect = paint_ctx
          .box_rect()
          .expect("when paint node, it's mut be already layout.");
        paint_ctx.painter.translate(rect.min_x(), rect.min_y());
        id.assert_get(self).paint(&mut paint_ctx);
      }

      w = id.first_child(self).filter(|_| need_paint).or_else(|| {
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
          let clamp = self
            .layout_info(*wid)
            .map(|info| info.clamp)
            .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });
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
          (l.performed_layout.borrow_mut())(LifeCycleCtx { id, tree: tree2 });
          true
        },
        QueryOrder::OutsideFirst,
      );
    });
  }

  pub(crate) fn set_root(&mut self, widget: Widget) {
    assert!(self.root.is_none());
    let theme = self.ctx.app_theme.clone();
    let root = widget
      .into_subtree(None, self, theme)
      .expect("must have a root");
    self.set_root_id(root);
    self.mark_dirty(root);
    root.on_mounted_subtree(self, &HashMap::default());
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

  pub(crate) fn app_ctx(&self) -> &AppContext { &self.ctx }

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

  pub(crate) fn swap_children(self, other: WidgetId, tree: &mut WidgetTree) {
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

  pub(crate) fn remove_subtree(
    self,
    tree: &mut WidgetTree,
    old_to_news: &HashMap<WidgetId, WidgetId>,
  ) {
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self
      .0
      .descendants(&tree1.arena)
      .map(WidgetId)
      .for_each(|id| {
        let disposed_type = old_to_news
          .get(&id)
          .map_or(DisposedType::Drop, |id| DisposedType::Replaced(*id));
        id.on_disposed(tree2, disposed_type);
      });
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

  pub(crate) fn on_mounted_subtree(
    self,
    tree: &mut WidgetTree,
    old_to_news: &HashMap<WidgetId, WidgetId>,
  ) {
    let (tree1, tree2) = unsafe { tree.split_tree() };
    let new_to_olds = old_to_news
      .iter()
      .map(|(old, new)| (*new, *old))
      .collect::<HashMap<WidgetId, WidgetId>>();
    self.descendants(tree1).for_each(|w| {
      let mount = new_to_olds
        .get(&w)
        .map_or(MountedType::New, |id| MountedType::Replace(*id));
      w.on_mounted(tree2, mount);
    });
  }

  pub(crate) fn on_mounted(self, tree: &mut WidgetTree, mounted_type: MountedType) {
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
    self.assert_get(tree1).query_all_type(
      |m: &MountedListener| {
        (m.mounted.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 }, mounted_type);
        true
      },
      QueryOrder::OutsideFirst,
    );
  }

  pub(crate) fn on_disposed(self, tree: &mut WidgetTree, disposed_type: DisposedType) {
    tree.layout_store.remove(&self);
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get(tree1).query_all_type(
      |d: &DisposedListener| {
        (d.disposed.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 }, disposed_type);
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

  pub(crate) fn is_offstage(self, tree: &WidgetTree) -> bool {
    let mut offstage = false;
    self
      .assert_get(tree)
      .query_on_first_type(QueryOrder::OutsideFirst, |r: &Offstage| {
        offstage = r.offstage
      });
    offstage
  }

  pub(crate) fn key(self, tree: &WidgetTree) -> Option<Key> {
    let mut key = None;
    self
      .assert_get(tree)
      .query_on_first_type(QueryOrder::OutsideFirst, |k: &Key| {
        key = Some(k.clone());
      });
    key
  }
}

impl Widget {
  pub(crate) fn into_subtree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
    current_theme: Rc<Theme>,
  ) -> Option<WidgetId> {
    enum NodeInfo {
      BackTheme(Rc<Theme>),
      Parent(WidgetId),
      Widget(Widget),
    }
    pub(crate) struct InflateHelper<'a> {
      stack: Vec<NodeInfo>,
      tree: &'a mut WidgetTree,
      current_theme: Rc<Theme>,
      parent: Option<WidgetId>,
      root: Option<WidgetId>,
    }

    impl<'a> InflateHelper<'a> {
      pub(crate) fn inflate(mut self, widget: Widget) -> Option<WidgetId> {
        self.place_node_in_tree(widget);
        loop {
          match self.stack.pop() {
            Some(NodeInfo::BackTheme(theme)) => {
              self.current_theme = theme;
            }
            Some(NodeInfo::Parent(p)) => self.parent = Some(p),
            Some(NodeInfo::Widget(w)) => {
              self.place_node_in_tree(w);
            }
            None => break,
          }
        }

        self.root
      }

      fn place_node_in_tree(&mut self, widget: Widget) {
        let Widget { node, children } = widget;
        let children_size = children.len();
        self.push_children(children);

        if let Some(node) = node {
          match node {
            WidgetNode::Compose(c) => {
              assert_eq!(children_size, 0, "compose widget shouldn't have child.");
              let mut build_ctx = BuildCtx::new(self.current_theme.clone(), self.tree);
              let c = c(&mut build_ctx);
              self
                .stack
                .push(NodeInfo::BackTheme(self.current_theme.clone()));
              self.current_theme = build_ctx.theme.clone();
              self.stack.push(NodeInfo::Widget(c));
            }
            WidgetNode::Render(r) => {
              let wid = self.tree.new_node(r);
              self.perpend(wid, children_size > 0);
            }
            WidgetNode::Dynamic(e) => {
              let w = Generator::new_generator(e, children_size > 0, self.tree);
              self.perpend(w, children_size > 0);
            }
          }
        } else {
          assert!(
            children_size <= 1,
            "None parent with multi child is forbidden."
          );
        }
      }

      fn push_children(&mut self, children: ChildVec<Widget>) {
        if let Some(p) = self.parent {
          self.stack.push(NodeInfo::Parent(p));
        }
        self
          .stack
          .extend(children.into_inner().into_iter().map(NodeInfo::Widget));
      }

      fn perpend(&mut self, child: WidgetId, has_child: bool) {
        if let Some(o) = self.parent {
          o.prepend(child, self.tree);
        }
        if has_child || self.parent.is_none() {
          self.parent = Some(child)
        }
        if self.root.is_none() {
          self.root = Some(child)
        }
      }
    }

    let helper = InflateHelper {
      stack: vec![],
      tree,
      current_theme,
      parent,
      root: None,
    };
    helper.inflate(self)
  }
}
#[cfg(test)]
mod tests {
  extern crate test;
  use crate::test::{
    expect_layout_result, layout_info_by_path, ExpectRect, LayoutTestItem, MockBox, MockMulti,
  };

  use super::*;
  use test::Bencher;

  #[derive(Clone, Debug)]
  pub struct Recursive {
    pub width: usize,
    pub depth: usize,
  }

  impl Compose for Recursive {
    fn compose(this: StateWidget<Self>) -> Widget {
      widget! {
        track { this: this.into_stateful() }
        MockMulti {
          ExprWidget {
            expr: (0..this.width)
              .map(move |_| {
                if this.depth > 1 {
                  Recursive {
                    width: this.width,
                    depth: this.depth - 1,
                  }.into_widget()
                } else {
                  MockBox { size: Size::new(10., 10.)}.into_widget()
                }
              })
          }
        }
      }
    }
  }

  #[derive(Clone, Debug)]
  pub struct Embed {
    pub width: usize,
    pub depth: usize,
  }

  impl Compose for Embed {
    fn compose(this: StateWidget<Self>) -> Widget {
      widget! {
        track { this: this.into_stateful()}
        MockMulti {
          ExprWidget {
            expr: (0..this.width - 1)
              .map(move |_| {
                MockBox { size: Size::new(10., 10.)}
              })
          }
          ExprWidget {
            expr: if this.depth > 1{
              Embed {
                width: this.width,
                depth: this.depth - 1,
              }.into_widget()
            } else {
              MockBox { size: Size::new(10., 10.)}.into_widget()
            }
          }
        }
      }
    }
  }

  fn bench_recursive_inflate(width: usize, depth: usize, b: &mut Bencher) {
    let ctx: AppContext = <_>::default();
    b.iter(move || {
      let mut tree = WidgetTree::new(Recursive { width, depth }.into_widget(), ctx.clone());
      tree.tree_repair();
    });
  }

  fn bench_recursive_repair(width: usize, depth: usize, b: &mut Bencher) {
    let w = Recursive { width, depth }.into_stateful();
    let trigger = w.clone();
    let mut tree = WidgetTree::new(w.into_widget(), <_>::default());
    b.iter(|| {
      {
        let mut v = trigger.state_ref();
        v.width = v.width;
      }
      tree.tree_repair();
    });
  }

  #[test]
  fn fix_relayout_incorrect_clamp() {
    let expect_size = Size::new(20., 20.);
    let no_boundary_size = INFINITY_SIZE.into_stateful();
    let w = widget! {
      track { size: no_boundary_size.clone() }
      MockBox {
        size: expect_size,
        MockBox { size: *size }
      }
    };
    let mut wnd = Window::default_mock(w, Some(Size::new(200., 200.)));
    wnd.draw_frame();
    let rect = layout_info_by_path(&wnd, &[0, 0]);
    assert_eq!(rect.size, expect_size);

    // when relayout the inner `MockBox`, its clamp should same with its previous
    // layout, and clamp its size.
    {
      *no_boundary_size.state_ref() = INFINITY_SIZE;
    }
    wnd.draw_frame();
    let rect = layout_info_by_path(&wnd, &[0, 0]);
    assert_eq!(rect.size, expect_size);
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
            MockBox {
              size: Size::zero(),
              ExprWidget { expr: child.then(|| Void )}
            }
          }
        })
      }
    };

    let mut wnd = Window::default_mock(w, None);
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

    let mut wnd = Window::default_mock(w, None);
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
    let post = Embed { width: 5, depth: 3 };
    let mut tree = WidgetTree::new(post.into_widget(), <_>::default());
    tree.tree_repair();
    assert_eq!(tree.count(), 16);

    tree.mark_dirty(tree.root());
    tree.root().remove_subtree(&mut tree, &HashMap::default());
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
    let ctx: AppContext = <_>::default();
    b.iter(move || {
      let post = Embed { width: 5, depth: 1000 };
      WidgetTree::new(post.into_widget(), ctx.clone());
    });
  }

  #[bench]
  fn inflate_50_pow_2(b: &mut Bencher) { bench_recursive_inflate(50, 2, b); }

  #[bench]
  fn inflate_100_pow_2(b: &mut Bencher) { bench_recursive_inflate(100, 2, b); }

  #[bench]
  fn inflate_10_pow_4(b: &mut Bencher) { bench_recursive_inflate(10, 4, b); }

  #[bench]
  fn inflate_10_pow_5(b: &mut Bencher) { bench_recursive_inflate(10, 5, b); }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let post = Embed { width: 5, depth: 1000 }.into_stateful();
    let trigger = post.clone();
    let mut tree = WidgetTree::new(post.into_widget(), <_>::default());
    b.iter(|| {
      {
        let mut v = trigger.state_ref();
        v.width = v.width;
      }
      tree.tree_repair();
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) { bench_recursive_repair(50, 2, b); }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) { bench_recursive_repair(100, 2, b); }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) { bench_recursive_repair(10, 4, b); }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) { bench_recursive_repair(10, 5, b); }

  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    let trigger = Stateful::new(1);
    let widget = widget! {
      track { trigger: trigger.clone() }
      MockMulti {
        ExprWidget {
          expr: (0..3).map(|_| if *trigger > 0 {
            MockBox { size: Size::new(1., 1.)}
          } else {
            MockBox { size: Size::zero()}
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
