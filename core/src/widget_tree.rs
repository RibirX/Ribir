use indextree::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

pub mod widget_id;
pub(crate) use widget_id::TreeArena;
pub use widget_id::WidgetId;
mod layout_info;
use crate::prelude::*;
pub use layout_info::*;

pub(crate) struct WidgetTree {
  arena: TreeArena,
  root: Option<WidgetId>,
  ctx: AppContext,
  state_changed: Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>,
  layout_store: LayoutStore,
}

impl WidgetTree {
  pub(crate) fn root(&self) -> WidgetId { self.root.expect("Empty tree.") }

  pub(crate) fn new_node(&mut self, node: Box<dyn Render>) -> WidgetId {
    WidgetId(self.arena.new_node(node))
  }

  pub(crate) fn new(root_widget: Widget, ctx: AppContext) -> WidgetTree {
    let mut tree = WidgetTree {
      arena: Arena::default(),
      root: None,
      state_changed: <_>::default(),
      ctx,
      layout_store: <_>::default(),
    };

    tree.set_root(root_widget);
    tree
  }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self, painter: &mut Painter) {
    let mut w = Some(self.root());

    fn paint_rect_intersect(painter: &mut Painter, rc: &Rect) -> bool {
      let paint_rect = painter.get_transform().outer_transformed_rect(rc);
      painter
        .visual_rect()
        .and_then(|rc| rc.intersection(&paint_rect))
        .is_some()
    }

    let mut paint_ctx = PaintingCtx::new(self.root(), self, painter);
    while let Some(id) = w {
      paint_ctx.id = id;
      paint_ctx.painter.save();

      let layout_box = paint_ctx
        .box_rect()
        .expect("when paint node, it's mut be already layout.");
      let render = id.assert_get(self);

      let need_paint = paint_ctx.painter.alpha() != 0.
        && (paint_rect_intersect(paint_ctx.painter, &layout_box) || render.can_overflow());

      if need_paint {
        paint_ctx
          .painter
          .translate(layout_box.min_x(), layout_box.min_y());
        render.paint(&mut paint_ctx);
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

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    let mut performed = vec![];

    loop {
      let Some(mut needs_layout) = self.layout_list() else {break;};
      while let Some(wid) = needs_layout.pop() {
        if wid.is_dropped(self) {
          continue;
        }

        let clamp = self
          .layout_info(wid)
          .map(|info| info.clamp)
          .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });

        self.perform_widget_layout(wid, clamp, &mut performed);
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
    }
  }

  /// Compute layout of the render widget `id`, and store its result in the
  /// store.
  pub(crate) fn perform_widget_layout(
    &mut self,
    widget: WidgetId,
    out_clamp: BoxClamp,
    performed: &mut Vec<WidgetId>,
  ) -> Size {
    self
      .layout_info(widget)
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

        widget.assert_get(tree1).query_all_type(
          |_: &PerformedLayoutListener| {
            performed.push(widget);
            false
          },
          QueryOrder::OutsideFirst,
        );
        size
      })
  }

  pub(crate) fn set_root(&mut self, widget: Widget) {
    assert!(self.root.is_none());
    let root = widget.into_subtree(None, self).expect("must have a root");
    self.set_root_id(root);
    self.mark_dirty(root);
    self.on_mounted_subtree(root);
  }

  pub(crate) fn set_root_id(&mut self, id: WidgetId) {
    assert!(self.root.is_none());
    self.root = Some(id);
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.state_changed.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { !self.state_changed.borrow().is_empty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self.arena).count() }

  pub(crate) fn arena(&self) -> &TreeArena { &self.arena() }

  pub(crate) fn layout_store(&self) -> &LayoutStore { &self.layout_store }

  pub(crate) fn app_ctx(&self) -> &AppContext { &self.ctx }

  #[allow(unused)]
  pub fn display_tree(&self, sub_tree: WidgetId) -> String {
    fn display_node(mut prefix: String, id: WidgetId, tree: &WidgetTree, display: &mut String) {
      display.push_str(&format!("{prefix}{:?}\n", id.0));

      prefix.pop();
      match prefix.pop() {
        Some('├') => prefix.push_str("│ "),
        Some(_) => prefix.push_str("  "),
        _ => {}
      }

      id.children(tree).for_each(|c| {
        let mut prefix = prefix.clone();
        let suffix = if Some(c) == id.last_child(tree) {
          "└─"
        } else {
          "├─"
        };
        prefix.push_str(suffix);
        display_node(prefix, c, tree, display)
      });
    }
    let mut display = String::new();
    display_node("".to_string(), sub_tree, self, &mut display);
    display
  }

  pub(crate) fn remove_subtree(self, tree: &mut TreeArena) {
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self
      .0
      .descendants(&tree1.arena)
      .map(WidgetId)
      .for_each(|id| {
        id.on_disposed(tree2);
      });
    self.0.remove_subtree(&mut tree1.arena);
    if tree1.root() == self {
      tree1.root.take();
    }
  }

  pub(crate) fn on_mounted_subtree(&mut self, tree: &mut TreeArena) {
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.descendants(tree1).for_each(|w| {
      w.on_mounted(tree2);
    });
  }

  pub(crate) fn on_widget_mounted(&self, w: WidgetId) {
    w.assert_get(&self.arena).query_all_type(
      |notifier: &StateChangeNotifier| {
        let state_changed = self.state_changed.clone();
        notifier
          .change_stream()
          .filter(|b| b.contains(ChangeScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(w);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    // Safety: lifecycle context have no way to change tree struct.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get(tree1).query_all_type(
      |m: &MountedListener| {
        (m.mounted.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 });
        true
      },
      QueryOrder::OutsideFirst,
    );
  }

  pub(crate) fn on_disposed(self, tree: &mut TreeArena) {
    tree.layout_store.remove(&self);
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get(tree1).query_all_type(
      |d: &DisposedListener| {
        (d.disposed.borrow_mut())(LifeCycleCtx { id: self, tree: tree2 });
        true
      },
      QueryOrder::OutsideFirst,
    )
  }
}

impl Widget {
  pub(crate) fn into_subtree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
  ) -> Option<WidgetId> {
    enum NodeInfo {
      BackTheme,
      Parent(WidgetId),
      Widget(Widget),
    }

    let mut themes = vec![];
    let full = parent.map_or(false, |p| {
      p.ancestors(tree).any(|p| {
        p.assert_get(tree).query_all_type(
          |t: &Rc<Theme>| {
            themes.push(t.clone());
            matches!(t.deref(), Theme::Inherit(_))
          },
          QueryOrder::InnerFirst,
        );
        matches!(themes.last().map(Rc::deref), Some(Theme::Full(_)))
      })
    });
    if !full {
      themes.push(tree.app_ctx().app_theme.clone());
    }

    pub(crate) struct InflateHelper<'a> {
      stack: Vec<NodeInfo>,
      tree: &'a mut WidgetTree,
      themes: RefCell<Vec<Rc<Theme>>>,
      parent: Option<WidgetId>,
      root: Option<WidgetId>,
    }

    impl<'a> InflateHelper<'a> {
      pub(crate) fn inflate(mut self, widget: Widget) -> Option<WidgetId> {
        self.place_node_in_tree(widget);
        loop {
          let Some(node) = self.stack.pop() else{ break};
          match node {
            NodeInfo::BackTheme => {
              self.themes.borrow_mut().pop();
            }
            NodeInfo::Parent(p) => self.parent = Some(p),
            NodeInfo::Widget(w) => {
              self.place_node_in_tree(w);
            }
          }
        }

        self.root
      }

      fn place_node_in_tree(&mut self, widget: Widget) {
        match widget {
          Widget::Compose(c) => {
            let theme_cnt = self.themes.borrow().len();
            let mut build_ctx = BuildCtx::new(&self.themes, self.tree);
            let c = c(&mut build_ctx);
            if theme_cnt < self.themes.borrow().len() {
              self.stack.push(NodeInfo::BackTheme);
            }
            self.stack.push(NodeInfo::Widget(c));
          }
          Widget::Render { render, children } => {
            let wid = self.tree.new_node(render);
            let children_size = children.len();
            self.push_children(children);
            self.perpend(wid, children_size > 0);
          }
        }
      }

      fn push_children(&mut self, children: Vec<Widget>) {
        if let Some(p) = self.parent {
          self.stack.push(NodeInfo::Parent(p));
        }
        self
          .stack
          .extend(children.into_iter().map(NodeInfo::Widget));
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
      themes: RefCell::new(themes),
      parent,
      root: None,
    };
    helper.inflate(self)
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use crate::test::{layout_info_by_path, MockBox, MockMulti};

  use super::*;
  use painter::{font_db::FontDB, shaper::TextShaper};
  use std::{sync::Arc, sync::RwLock};
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
          DynWidget {
            dyns: (0..this.width)
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
              .collect::<Vec<_>>()
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
          DynWidget {
            dyns: (0..this.width - 1)
              .map(move |_| {
                MockBox { size: Size::new(10., 10.)}
              }).collect::<Vec<_>>()
          }
          DynWidget {
            dyns: if this.depth > 1{
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
      tree.layout(Size::new(512., 512.));
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
      tree.layout(Size::new(512., 512.));
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
      DynWidget {
        dyns: parent.then(|| {
          widget!{
            MockBox {
              size: Size::zero(),
              DynWidget { dyns: child.then(|| Void )}
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
      DynWidget {
        dyns: trigger.then(|| {
          widget!{ DynWidget { dyns: trigger.then(|| Void )}}
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
    tree.layout(Size::new(512., 512.));
    assert_eq!(tree.count(), 16);

    tree.mark_dirty(tree.root());
    tree.root().remove_subtree(&mut tree);
    assert_eq!(tree.layout_list(), None);
    assert_eq!(tree.is_dirty(), false);
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
      tree.layout(Size::new(512., 512.));
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
        DynWidget {
          dyns: (0..3).map(|_| if *trigger > 0 {
            MockBox { size: Size::new(1., 1.)}
          } else {
            MockBox { size: Size::zero()}
          }).collect::<Vec<_>>()
        }
      }
    };

    let mut tree = WidgetTree::new(widget, <_>::default());
    tree.layout(Size::new(100., 100.));
    {
      *trigger.silent_ref() = 2;
    }
    assert_eq!(tree.is_dirty(), false)
  }

  #[test]
  fn draw_clip() {
    let mut font_db = FontDB::default();
    font_db.load_system_fonts();
    let font_db = Arc::new(RwLock::new(font_db));
    let shaper = TextShaper::new(font_db.clone());
    let store = TypographyStore::new(<_>::default(), font_db.clone(), shaper);
    let win_size = Size::new(150., 50.);
    let mut painter = Painter::new(2., store, win_size);

    let w1 = widget! {
       MockMulti {
        DynWidget {
          dyns: (0..100).map(|_|
            widget! {MockBox {
             size: Size::new(150., 50.),
             background: Color::BLUE,
          }}).collect::<Vec<_>>()
        }
    }};
    let mut tree1 = WidgetTree::new(w1, <_>::default());
    tree1.layout(win_size);
    tree1.draw(&mut painter);

    let len_100_widget = painter.finish().len();

    let w2 = widget! {
       MockMulti {
        DynWidget {
          dyns: (0..1).map(|_|
            widget! { MockBox {
             size: Size::new(150., 50.),
             background: Color::BLUE,
          }}).collect::<Vec<_>>()
        }
    }};
    let mut tree2 = WidgetTree::new(w2, <_>::default());
    tree2.layout(win_size);
    tree2.draw(&mut painter);
    let len_1_widget = painter.finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }
}
