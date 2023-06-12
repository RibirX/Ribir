use std::{
  cell::RefCell,
  cmp::Reverse,
  collections::HashSet,
  rc::{Rc, Weak},
};

pub mod widget_id;
pub(crate) use widget_id::TreeArena;
pub use widget_id::WidgetId;
mod layout_info;
use crate::prelude::*;
pub use layout_info::*;

pub(crate) type DirtySet = Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>;

#[derive(Default)]
pub(crate) struct WidgetTree {
  pub(crate) root: Option<WidgetId>,
  wnd: Weak<Window>,
  pub(crate) arena: TreeArena,
  pub(crate) store: LayoutStore,
  pub(crate) dirty_set: DirtySet,
}

impl WidgetTree {
  pub(crate) fn new() -> WidgetTree { Self::default() }

  pub fn init(&mut self, widget: Widget, wnd: Weak<Window>) {
    self.wnd = wnd;
    let build_ctx = BuildCtx::new(None, self);
    let root = widget.build(&build_ctx);
    self.root = Some(root);
    self.mark_dirty(root);
    root.on_mounted_subtree(self);
  }

  pub(crate) fn root(&self) -> WidgetId {
    self.root.expect("Try to access a not init `WidgetTree`")
  }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self) {
    let wnd = self.window();
    let mut ctx = PaintingCtx::new(self.root(), &wnd);
    self.root().paint_subtree(&mut ctx);
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

        let wnd = self.window();
        let wnd = &*wnd;

        let mut layouter = Layouter::new(wid, wnd, true, self);
        layouter.perform_widget_layout(clamp);
      }
    }
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.dirty_set.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { !self.dirty_set.borrow().is_empty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self.arena).count() }

  pub(crate) fn window(&self) -> Rc<Window> {
    self
      .wnd
      .upgrade()
      .expect("The window of `FocusManager` has already dropped.")
  }

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

  pub(crate) fn layout_list(&mut self) -> Option<Vec<WidgetId>> {
    if self.dirty_set.borrow().is_empty() {
      return None;
    }

    let mut needs_layout = vec![];

    let dirty_widgets = {
      let mut state_changed = self.dirty_set.borrow_mut();
      let dirty_widgets = state_changed.clone();
      state_changed.clear();
      dirty_widgets
    };

    for id in dirty_widgets.iter() {
      if id.is_dropped(&self.arena) {
        continue;
      }

      let mut relayout_root = *id;
      if let Some(info) = self.store.get_mut(id) {
        info.size.take();
      }

      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        if self.store.layout_box_size(p).is_none() {
          break;
        }

        relayout_root = p;
        if let Some(info) = self.store.get_mut(&p) {
          info.size.take();
        }

        let r = self.arena.get(p.0).unwrap().get();
        if r.only_sized_by_parent() {
          break;
        }
      }
      needs_layout.push(relayout_root);
    }

    (!needs_layout.is_empty()).then(|| {
      needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(&self.arena).count()));
      needs_layout
    })
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use crate::test_helper::{MockBox, MockMulti, TestWindow};

  use super::*;
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
          Multi::new((0..this.width)
          .map(move |_| {
            if this.depth > 1 {
              Widget::from(
                Recursive {
                  width: this.width,
                  depth: this.depth - 1,
                }
              )
            } else {
              Widget::from(MockBox { size: Size::new(10., 10.)})
            }
          }))
        }
      }
      .into()
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
          Multi::new((0..this.width - 1)
          .map(move |_| {
            MockBox { size: Size::new(10., 10.)}
          }))
          DynWidget {
            dyns: if this.depth > 1 {
              Widget::from(Embed {
                width: this.width,
                depth: this.depth - 1,
              })
            } else {
              Widget::from(MockBox { size: Size::new(10., 10.)})
            }
          }
        }
      }
      .into()
    }
  }

  fn bench_recursive_inflate(width: usize, depth: usize, b: &mut Bencher) {
    let wnd = TestWindow::new(Void {});
    b.iter(move || {
      let mut tree = wnd.widget_tree.borrow_mut();
      tree.init(Recursive { width, depth }.into(), Rc::downgrade(&wnd.0));
      tree.layout(Size::new(512., 512.));
    });
  }

  fn bench_recursive_repair(width: usize, depth: usize, b: &mut Bencher) {
    let w = Stateful::new(Recursive { width, depth });
    let trigger = w.clone();
    let wnd = TestWindow::new(w);
    let mut tree = wnd.widget_tree.borrow_mut();
    b.iter(|| {
      {
        let _: &mut Recursive = &mut trigger.state_ref();
      }
      tree.layout(Size::new(512., 512.));
    });
  }

  #[test]
  fn fix_relayout_incorrect_clamp() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let expect_size = Size::new(20., 20.);
    let no_boundary_size = Stateful::new(INFINITY_SIZE);
    let w = widget! {
      states { size: no_boundary_size.clone() }
      MockBox {
        size: expect_size,
        MockBox { size: *size }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    let size = wnd.layout_info_by_path(&[0, 0]).unwrap().size.unwrap();
    assert_eq!(size, expect_size);

    // when relayout the inner `MockBox`, its clamp should same with its previous
    // layout, and clamp its size.
    {
      *no_boundary_size.state_ref() = INFINITY_SIZE;
    }
    wnd.draw_frame();
    let size = wnd.layout_info_by_path(&[0, 0]).unwrap().size.unwrap();
    assert_eq!(size, expect_size);
  }

  #[test]
  fn fix_dropped_child_expr_widget() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let parent = Stateful::new(true);
    let child = Stateful::new(true);
    let w = widget! {
      states { parent: parent.clone(), child: child.clone() }
      widget::then(*parent, || widget!{
        MockBox {
          size: Size::zero(),
          widget::then(*child, || Void)
        }
      })
    };

    let mut wnd = TestWindow::new(w);
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let trigger = Stateful::new(true);
    let w = widget! {
      states { trigger: trigger.clone() }
      widget::then(*trigger, || widget!{
        widget::then(*trigger, || Void)
      })
    };

    let mut wnd = TestWindow::new(w);
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let post = Embed { width: 5, depth: 3 };
    let wnd = TestWindow::new(post);
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::new(512., 512.));
    assert_eq!(tree.count(), 16);

    let root = tree.root();
    tree.mark_dirty(root);

    root.remove_subtree(&mut tree);

    assert_eq!(tree.layout_list(), None);
    assert!(!tree.is_dirty());
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    let wnd = TestWindow::new(Void);

    b.iter(move || {
      let post = Embed { width: 5, depth: 1000 };
      WidgetTree::new().init(post.into(), Rc::downgrade(&wnd.0));
    });
  }

  #[bench]
  fn inflate_50_pow_2(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_inflate(50, 2, b);
  }

  #[bench]
  fn inflate_100_pow_2(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_inflate(100, 2, b);
  }

  #[bench]
  fn inflate_10_pow_4(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_inflate(10, 4, b);
  }

  #[bench]
  fn inflate_10_pow_5(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_inflate(10, 5, b);
  }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    let post = Stateful::new(Embed { width: 5, depth: 1000 });
    let trigger = post.clone();
    let wnd = TestWindow::new(post);
    let mut tree = wnd.widget_tree.borrow_mut();

    b.iter(|| {
      {
        let _: &mut Embed = &mut trigger.state_ref();
      }
      tree.layout(Size::new(512., 512.));
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_repair(50, 2, b);
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_repair(100, 2, b);
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_repair(10, 4, b);
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    bench_recursive_repair(10, 5, b);
  }

  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let trigger = Stateful::new(1);
    let widget = widget! {
      states { trigger: trigger.clone() }
      MockMulti {
        Multi::new((0..3).map(move |_| if *trigger > 0 {
          MockBox { size: Size::new(1., 1.)}
        } else {
          MockBox { size: Size::zero()}
        }))
      }
    };

    let wnd = TestWindow::new(widget);
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::new(100., 100.));
    {
      *trigger.silent_ref() = 2;
    }
    assert!(!tree.is_dirty())
  }

  #[test]
  fn draw_clip() {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    let win_size = Size::new(150., 50.);

    let w1 = widget! {
       MockMulti {
        Multi::new((0..100).map(|_|
          widget! { MockBox {
           size: Size::new(150., 50.),
           background: Color::BLUE,
        }}))

    }};
    let mut wnd = TestWindow::new_with_size(w1, win_size);
    wnd.draw_frame();

    let len_100_widget = wnd.painter.borrow_mut().finish().len();

    let w2 = widget! {
      MockMulti {
        Multi::new((0..1).map(|_|
          widget! { MockBox {
           size: Size::new(150., 50.),
           background: Color::BLUE,
        }}))
    }};

    let mut wnd = TestWindow::new(w2);
    wnd.draw_frame();
    let len_1_widget = wnd.painter.borrow_mut().finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }
}
