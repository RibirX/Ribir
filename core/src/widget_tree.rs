use indextree::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

pub mod widget_id;
pub(crate) use widget_id::TreeArena;
pub use widget_id::WidgetId;
mod layout_info;
use crate::{prelude::*, widget::widget_id::new_node};
pub use layout_info::*;

pub(crate) type DirtySet = Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>;
pub(crate) struct WidgetTree {
  root: WidgetId,
  pub(crate) arena: TreeArena,
  pub(crate) store: LayoutStore,
  pub(crate) wnd_ctx: WindowCtx,
  dirty_set: DirtySet,
}

impl WidgetTree {
  pub(crate) fn new(root: Widget, wnd_ctx: WindowCtx) -> WidgetTree {
    let mut arena = Arena::default();
    let root = root
      .into_subtree(None, &mut arena, &wnd_ctx)
      .expect("must have a root");
    let store = LayoutStore::default();
    let dirty_set = DirtySet::default();
    root.on_mounted_subtree(&arena, &store, &wnd_ctx, &dirty_set);
    let tree = Self {
      root,
      arena,
      wnd_ctx,
      store,
      dirty_set,
    };
    tree.mark_dirty(root);
    tree
  }

  pub(crate) fn root(&self) -> WidgetId { self.root }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self, painter: &mut Painter) {
    fn paint_rect_intersect(painter: &mut Painter, rc: &Rect) -> bool {
      let paint_rect = painter.get_transform().outer_transformed_rect(rc);
      painter
        .visual_rect()
        .and_then(|rc| rc.intersection(&paint_rect))
        .is_some()
    }

    let Self { root, arena, store, wnd_ctx, .. } = self;
    let mut w = Some(*root);

    let mut paint_ctx = PaintingCtx {
      id: *root,
      arena,
      store,
      wnd_ctx,
      painter,
    };
    while let Some(id) = w {
      paint_ctx.id = id;
      paint_ctx.painter.save();

      let mut need_paint = false;
      if paint_ctx.painter.alpha() != 0. {
        if let Some(layout_box) = paint_ctx.box_rect() {
          let render = id.assert_get(arena);
          if paint_rect_intersect(paint_ctx.painter, &layout_box) || render.can_overflow() {
            paint_ctx
              .painter
              .translate(layout_box.min_x(), layout_box.min_y());
            render.paint(&mut paint_ctx);
            need_paint = true;
          }
        };
      }

      w = id.first_child(arena).filter(|_| need_paint).or_else(|| {
        let mut node = w;
        while let Some(p) = node {
          // self node sub-tree paint finished, goto sibling
          paint_ctx.painter.restore();
          node = p.next_sibling(arena);
          if node.is_some() {
            break;
          } else {
            // if there is no more sibling, back to parent to find sibling.
            node = p.parent(arena);
          }
        }
        node
      });
    }
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    loop {
      let Some(mut needs_layout) = self.layout_list() else {break;};
      while let Some(wid) = needs_layout.pop() {
        if wid.is_dropped(&self.arena) {
          continue;
        }

        let clamp = self
          .store
          .layout_info(wid)
          .map(|info| info.clamp)
          .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });

        let Self { arena, store, wnd_ctx, dirty_set, .. } = self;
        let mut layouter = Layouter {
          wid,
          arena,
          store,
          wnd_ctx,
          dirty_set,
        };
        layouter.perform_widget_layout(clamp);

        store.take_performed().into_iter().for_each(|id| {
          id.assert_get(arena).query_all_type(
            |l: &PerformedLayoutListener| {
              l.dispatch(LifeCycleCtx { id, arena, store, wnd_ctx });
              true
            },
            QueryOrder::OutsideFirst,
          );
        });
      }
    }
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.dirty_set.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { !self.dirty_set.borrow().is_empty() }

  pub(crate) fn count(&self) -> usize { self.root.descendants(&self.arena).count() }

  #[allow(unused)]
  pub fn display_tree(&self, sub_tree: WidgetId) -> String {
    fn display_node(mut prefix: String, id: WidgetId, tree: &TreeArena, display: &mut String) {
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
    display_node("".to_string(), sub_tree, &self.arena, &mut display);
    display
  }
}

impl Widget {
  pub(crate) fn into_subtree(
    self,
    parent: Option<WidgetId>,
    arena: &mut TreeArena,
    wnd_ctx: &WindowCtx,
  ) -> Option<WidgetId> {
    enum NodeInfo {
      BackTheme,
      Parent(WidgetId),
      Widget(Widget),
    }

    let mut themes = vec![];
    let full = parent.map_or(false, |p| {
      p.ancestors(arena).any(|p| {
        p.assert_get(arena).query_all_type(
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
      themes.push(wnd_ctx.app_theme());
    }

    pub(crate) struct InflateHelper<'a> {
      stack: Vec<NodeInfo>,
      arena: &'a mut TreeArena,
      wnd_ctx: &'a WindowCtx,
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
            let mut build_ctx = BuildCtx::new(&self.themes, self.wnd_ctx);
            let c = c(&mut build_ctx);
            if theme_cnt < self.themes.borrow().len() {
              self.stack.push(NodeInfo::BackTheme);
            }
            self.stack.push(NodeInfo::Widget(c));
          }
          Widget::Render { render, children } => {
            let wid = new_node(self.arena, render);
            if let Some(children) = children {
              let children_size = children.len();
              self.push_children(children);
              self.perpend(wid, children_size > 0);
            } else {
              self.perpend(wid, false);
            }
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
          o.prepend(child, self.arena);
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
      arena,
      wnd_ctx,
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
  use crate::{
    test::{layout_info_by_path, MockBox, MockMulti},
    widget::widget_id::dispose_nodes,
  };

  use super::*;
  use ribir_painter::{font_db::FontDB, shaper::TextShaper};
  use std::{sync::Arc, sync::RwLock};
  use test::Bencher;

  #[derive(Clone, Debug)]
  pub struct Recursive {
    pub width: usize,
    pub depth: usize,
  }

  impl Compose for Recursive {
    fn compose(this: State<Self>) -> Widget {
      widget! {
        states { this: this.into_writable() }
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
    fn compose(this: State<Self>) -> Widget {
      widget! {
        states { this: this.into_writable()}
        MockMulti {
          DynWidget {
            dyns: (0..this.width - 1)
              .map(move |_| {
                MockBox { size: Size::new(10., 10.)}
              })
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
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    b.iter(move || {
      let mut tree = WidgetTree::new(
        Recursive { width, depth }.into_widget(),
        WindowCtx::new(ctx.clone(), scheduler.clone()),
      );
      tree.layout(Size::new(512., 512.));
    });
  }

  fn bench_recursive_repair(width: usize, depth: usize, b: &mut Bencher) {
    let w = Stateful::new(Recursive { width, depth });
    let trigger = w.clone();
    let app_ctx = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w.into_widget(), WindowCtx::new(app_ctx, scheduler));
    b.iter(|| {
      {
        let _: &mut Recursive = &mut trigger.state_ref();
      }
      tree.layout(Size::new(512., 512.));
    });
  }

  #[test]
  fn fix_relayout_incorrect_clamp() {
    let expect_size = Size::new(20., 20.);
    let no_boundary_size = Stateful::new(INFINITY_SIZE);
    let w = widget! {
      states { size: no_boundary_size.clone() }
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
      states { parent: parent.clone(), child: child.clone() }
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
      states { trigger: trigger.clone() }
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
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(
      post.into_widget(),
      WindowCtx::new(AppContext::default(), scheduler),
    );
    tree.layout(Size::new(512., 512.));
    assert_eq!(tree.count(), 16);

    tree.mark_dirty(tree.root());
    let WidgetTree { root, arena, store, wnd_ctx, .. } = &mut tree;

    dispose_nodes(root.descendants(arena), arena, store, wnd_ctx);
    root.remove_subtree(arena);

    assert_eq!(tree.layout_list(), None);
    assert!(!tree.is_dirty());
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    let ctx: AppContext = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    b.iter(move || {
      let post = Embed { width: 5, depth: 1000 };
      WidgetTree::new(
        post.into_widget(),
        WindowCtx::new(ctx.clone(), scheduler.clone()),
      );
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
    let post = Stateful::new(Embed { width: 5, depth: 1000 });
    let trigger = post.clone();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(
      post.into_widget(),
      WindowCtx::new(AppContext::default(), scheduler),
    );
    b.iter(|| {
      {
        let _: &mut Embed = &mut trigger.state_ref();
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
      states { trigger: trigger.clone() }
      MockMulti {
        DynWidget {
          dyns: (0..3).map(move |_| if *trigger > 0 {
            MockBox { size: Size::new(1., 1.)}
          } else {
            MockBox { size: Size::zero()}
          })
        }
      }
    };

    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(widget, WindowCtx::new(AppContext::default(), scheduler));
    tree.layout(Size::new(100., 100.));
    {
      *trigger.silent_ref() = 2;
    }
    assert!(!tree.is_dirty())
  }

  #[test]
  fn draw_clip() {
    let mut font_db = FontDB::default();
    font_db.load_system_fonts();
    let font_db = Arc::new(RwLock::new(font_db));
    let shaper = TextShaper::new(font_db.clone());
    let store = TypographyStore::new(<_>::default(), font_db, shaper);
    let win_size = Size::new(150., 50.);
    let mut painter = Painter::new(2., store, win_size);

    let w1 = widget! {
       MockMulti {
        DynWidget {
          dyns: (0..100).map(|_|
            widget! { MockBox {
             size: Size::new(150., 50.),
             background: Color::BLUE,
          }})
        }
    }};
    let app_ctx = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree1 = WidgetTree::new(w1, WindowCtx::new(app_ctx, scheduler));
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
          }})
        }
    }};
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree2 = WidgetTree::new(w2, WindowCtx::new(AppContext::default(), scheduler));
    tree2.layout(win_size);
    tree2.draw(&mut painter);
    let len_1_widget = painter.finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }
}
