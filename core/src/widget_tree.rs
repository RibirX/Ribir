use std::{
  cell::RefCell,
  cmp::Reverse,
  collections::HashSet,
  mem::MaybeUninit,
  rc::{Rc, Weak},
};

pub mod widget_id;
use indextree::Arena;
use widget_id::RenderQueryable;
pub use widget_id::WidgetId;
mod layout_info;
pub use layout_info::*;

use self::widget::widget_id::new_node;
use crate::{overlay::OverlayRoot, prelude::*, render_helper::PureRender};

pub(crate) type DirtySet = Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>;

pub(crate) struct WidgetTree {
  pub(crate) root: WidgetId,
  wnd: Weak<Window>,
  pub(crate) arena: TreeArena,
  pub(crate) store: LayoutStore,
  pub(crate) dirty_set: DirtySet,
}

type TreeArena = Arena<Box<dyn RenderQueryable>>;

impl WidgetTree {
  pub fn init(&mut self, wnd: &Rc<Window>, content: GenWidget) {
    self.wnd = Rc::downgrade(wnd);
    let root_id = self.root;
    let mut ctx = BuildCtx::create(self.root, wnd.tree);
    ctx.pre_alloc = Some(root_id);

    let theme = AppCtx::app_theme().clone_writer();
    let overlays = Queryable(Overlays::default());
    let id = Provider::new(Box::new(overlays))
      .with_child(fn_widget! {
        theme.with_child(fn_widget!{
          @Root {
            @{ content.gen_widget() }
            @{
              let overlays = Provider::of::<Overlays>(ctx!()).unwrap();
              overlays.rebuild()
            }
          }
        })
      })
      .into_widget()
      .build(&mut ctx);

    assert_eq!(root_id, id);

    self.mark_dirty(root_id);
    root_id.on_mounted_subtree(self);
  }

  pub(crate) fn root(&self) -> WidgetId { self.root }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self) {
    let wnd = self.window();
    let mut painter = wnd.painter.borrow_mut();
    let tree = wnd.tree();
    let mut ctx = PaintingCtx::new(self.root(), tree, &mut painter);
    self.root().paint_subtree(&mut ctx);
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    loop {
      let Some(mut needs_layout) = self.layout_list() else {
        break;
      };
      while let Some(wid) = needs_layout.pop() {
        if wid.is_dropped(self) {
          continue;
        }

        let clamp = self
          .store
          .layout_info(wid)
          .map(|info| info.clamp)
          .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });

        let mut layouter = Layouter::new(wid, true, self);
        layouter.perform_widget_layout(clamp);
      }
    }
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.dirty_set.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { !self.dirty_set.borrow().is_empty() }

  pub(crate) fn count(&self, wid: WidgetId) -> usize { wid.descendants(self).count() }

  pub(crate) fn window(&self) -> Rc<Window> {
    self
      .wnd
      .upgrade()
      .expect("Must initialize the widget tree before use it.")
  }

  #[allow(unused)]
  pub fn display_tree(&self, sub_tree: WidgetId) -> String {
    let mut display = String::new();
    self.display_node("".to_string(), sub_tree, &mut display);
    display
  }

  fn display_node(&self, mut prefix: String, id: WidgetId, display: &mut String) {
    display.push_str(&format!("{prefix}{:?}\n", id.0));

    prefix.pop();
    match prefix.pop() {
      Some('├') => prefix.push_str("│ "),
      Some(_) => prefix.push_str("  "),
      _ => {}
    }

    id.children(self).for_each(|c| {
      let mut prefix = prefix.clone();
      let suffix = if Some(c) == id.last_child(self) { "└─" } else { "├─" };
      prefix.push_str(suffix);
      self.display_node(prefix, c, display)
    });
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
      if id.is_dropped(self) {
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

        let r = p.assert_get(self);
        if r.only_sized_by_parent() {
          break;
        }
      }
      needs_layout.push(relayout_root);
    }

    (!needs_layout.is_empty()).then(|| {
      needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(self).count()));
      needs_layout
    })
  }

  pub fn detach(&mut self, id: WidgetId) {
    if self.root() == id {
      let root = self.root();
      let new_root = root
        .next_sibling(self)
        .or_else(|| root.prev_sibling(self))
        .expect("Try to remove the root and there is no other widget can be the new root.");
      self.root = new_root;
    }

    id.0.detach(&mut self.arena);
  }

  pub(crate) fn remove_subtree(&mut self, id: WidgetId) {
    assert_ne!(id, self.root(), "You should detach the root widget before remove it.");

    id.0.descendants(&self.arena).for_each(|id| {
      self.store.remove(WidgetId(id));
    });
    id.0.remove_subtree(&mut self.arena);
  }

  pub(crate) fn get_many_mut<const N: usize>(
    &mut self, ids: &[WidgetId; N],
  ) -> [&mut Box<dyn RenderQueryable>; N] {
    unsafe {
      let mut outs: MaybeUninit<[&mut Box<dyn RenderQueryable>; N]> = MaybeUninit::uninit();
      let outs_ptr = outs.as_mut_ptr();
      for (idx, wid) in ids.iter().enumerate() {
        let tree = &mut *(self as *mut Self);
        let cur = wid
          .get_node_mut(tree)
          .expect("Invalid widget id.");

        *(*outs_ptr).get_unchecked_mut(idx) = cur;
      }
      outs.assume_init()
    }
  }
}

impl Default for WidgetTree {
  fn default() -> Self {
    let mut arena = TreeArena::new();

    Self {
      root: new_node(&mut arena, Box::new(PureRender(OverlayRoot {}))),
      wnd: Weak::new(),
      arena,
      store: LayoutStore::default(),
      dirty_set: Rc::new(RefCell::new(HashSet::default())),
    }
  }
}

#[simple_declare]
#[derive(MultiChild)]
pub(crate) struct Root;

impl Render for Root {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let mut layouter = ctx.first_child_layouter();
    while let Some(mut l) = layouter {
      let child_size = l.perform_widget_layout(clamp);
      size = size.max(child_size);
      layouter = l.into_next_sibling();
    }
    size
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

pub(crate) struct Overlays(Stateful<Vec<Overlay>>);

impl Overlays {
  fn rebuild(&self) -> Vec<Widget<'static>> {
    // todo: drop the old overlay subtree, create a new one for new theme
    vec![]
  }
}

impl Default for Overlays {
  fn default() -> Self { Self(Stateful::new(vec![])) }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[cfg(target_arch = "wasm32")]
  use crate::test_helper::wasm_bindgen_test;
  use crate::{
    reset_test_env,
    test_helper::{MockBox, MockMulti, TestWindow},
  };

  impl WidgetTree {
    pub(crate) fn content_root(&self) -> WidgetId { self.root.first_child(self).unwrap() }
  }

  fn empty_node(arena: &mut TreeArena) -> WidgetId { new_node(arena, Box::new(PureRender(Void))) }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn fix_relayout_incorrect_clamp() {
    reset_test_env!();

    let expect_size = Size::new(20., 20.);
    let no_boundary_size = Stateful::new(INFINITY_SIZE);
    let c_size = no_boundary_size.clone_writer();
    let w = fn_widget! {
      @MockBox {
        size: expect_size,
        @MockBox { size: pipe!(*$no_boundary_size) }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    let size = wnd
      .layout_info_by_path(&[0, 0])
      .unwrap()
      .size
      .unwrap();
    assert_eq!(size, expect_size);

    // when relayout the inner `MockBox`, its clamp should same with its previous
    // layout, and clamp its size.
    {
      *c_size.write() = INFINITY_SIZE;
    }
    wnd.draw_frame();
    let size = wnd
      .layout_info_by_path(&[0, 0])
      .unwrap()
      .size
      .unwrap();
    assert_eq!(size, expect_size);
  }

  #[test]
  fn fix_dropped_child_expr_widget() {
    reset_test_env!();

    let parent = Stateful::new(true);
    let child = Stateful::new(true);
    let c_p = parent.clone_writer();
    let c_c = child.clone_writer();
    let w = fn_widget! {
      @ {
        pipe!(*$parent).map(move |p|{
          if p {
            @MockBox {
              size: Size::zero(),
              @ { pipe!($child.then(|| Void)) }
            }.into_widget()
          } else {
            Void.into_widget()
          }
        })
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    {
      *c_c.write() = false;
      *c_p.write() = false;
    }

    // fix crash here.
    wnd.draw_frame();
  }

  #[test]
  fn fix_child_expr_widget_same_root_as_parent() {
    reset_test_env!();

    let trigger = Stateful::new(true);
    let c_trigger = trigger.clone_writer();
    let w = fn_widget! { @ { pipe!($trigger; Void) }};

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    {
      *c_trigger.write() = false;
    }

    // fix crash here
    // crash because generator live as long as its parent, at here two expr widget's
    // parent both none, all as root expr widget, parent expr widget can't remove
    // child expr widget.
    //
    // generator lifetime should bind to its generator widget instead of parent.
    wnd.draw_frame();
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn drop_info_clear() {
    reset_test_env!();

    let wnd = TestWindow::new(fn_widget! {
      @MockMulti {
        @ {
          (1..=10).map(|_| {
            let size = Size::new(10., 10.);
            MockBox { size }
          })
        }
      }
    });
    let tree = wnd.tree_mut();
    tree.layout(Size::new(512., 512.));
    assert_eq!(tree.count(tree.content_root()), 11);

    let root = tree.root();
    tree.mark_dirty(root);
    let new_root = empty_node(&mut tree.arena);
    root.insert_after(new_root, tree);
    tree.mark_dirty(new_root);
    tree.detach(root);
    tree.remove_subtree(root);

    assert_eq!(tree.layout_list(), Some(vec![new_root]));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    reset_test_env!();

    let trigger = Stateful::new(1);
    let c_trigger = trigger.clone_writer();
    let widget = fn_widget! {
      @ MockMulti {
        @{
          pipe!(*$trigger).map(move |b| {
            let size = if b > 0 {
              Size::new(1., 1.)
            } else {
              Size::zero()
            };
            (0..3).map(move |_| MockBox { size })
          })
        }
      }
    };

    let wnd = TestWindow::new(widget);
    let tree = wnd.tree_mut();
    tree.layout(Size::new(100., 100.));

    *c_trigger.write() = 2;
    assert!(!tree.is_dirty())
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn draw_clip() {
    reset_test_env!();

    let win_size = Size::new(150., 50.);

    let w1 = fn_widget! {
       @MockMulti {
        @ {
          (0..100).map(|_|
            @MockBox {
              size: Size::new(150., 50.),
              background: Color::BLUE,
          })
        }
    }};
    let mut wnd = TestWindow::new_with_size(w1, win_size);
    wnd.draw_frame();

    let len_100_widget = wnd.painter.borrow_mut().finish().len();

    let w2 = fn_widget! {
      @MockMulti {
        @ {
          (0..1).map(|_|
            @MockBox {
             size: Size::new(150., 50.),
             background: Color::BLUE,
          })
        }
    }};

    let mut wnd = TestWindow::new(w2);
    wnd.draw_frame();
    let len_1_widget = wnd.painter.borrow_mut().finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }
}
