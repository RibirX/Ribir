use std::{cell::RefCell, cmp::Reverse, collections::BTreeSet, mem::MaybeUninit};

pub mod widget_id;
use indextree::Arena;
pub use widget_id::{RenderQueryable, TrackId, WidgetId};
mod layout_info;
pub use layout_info::*;

use self::widget::widget_id::new_node;
use crate::{overlay::ShowingOverlays, prelude::*, render_helper::PureRender, window::WindowId};

/// This enum defines the dirty phases of the widget.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DirtyPhase {
  /// Indicates that the widget requires a layout update.
  Layout,
  /// This indicates that the subtree needs to undergo a forced relayout,
  /// primarily used for providers that may introduce layout constraints beyond
  /// the parent level.
  LayoutSubtree,
  /// Indicates that only the widget's position needs updating (no size change).
  /// When a widget with this flag is marked dirty with `DirtyPhase::Position`,
  /// only `place_children` is called instead of a full layout.
  Position,
  /// Indicates that the widget needs to be repainted.
  Paint,
}

impl DirtyPhase {
  /// Returns the priority of this dirty phase. Higher values mean broader
  /// invalidation scope and take precedence when merging.
  fn priority(self) -> u8 {
    match self {
      DirtyPhase::Paint => 0,
      DirtyPhase::Position => 1,
      DirtyPhase::Layout => 2,
      DirtyPhase::LayoutSubtree => 3,
    }
  }
}

pub(crate) type DirtySet = Rc<RefCell<ahash::HashMap<WidgetId, DirtyPhase>>>;

pub(crate) struct WidgetTree {
  pub(crate) root: WidgetId,
  pub(crate) wnd_id: WindowId,
  pub(crate) arena: TreeArena,
  pub(crate) store: LayoutStore,
  pub(crate) detached_parent: ahash::HashMap<WidgetId, WidgetId>,
  pub(crate) dirty_set: DirtySet,
  pub(crate) dummy_id: WidgetId,
}

/// A tool that help you to mark a widget as dirty
#[derive(Clone)]
pub(crate) struct DirtyMarker(DirtySet);

type TreeArena = Arena<Box<dyn RenderQueryable>>;

impl WidgetTree {
  pub fn init(&mut self, wnd: &Window, content: GenWidget) -> WidgetId {
    self.root.0.remove_subtree(&mut self.arena);
    self.detached_parent.clear();
    let _guard = BuildCtx::init(BuildCtx::empty(wnd.tree));

    let theme = AppCtx::app_theme().clone_writer();
    let child = GenWidget::new(move || {
      let overlays = Provider::of::<ShowingOverlays>(BuildCtx::get()).unwrap();
      overlays.rebuild();
      Root.with_child(content.gen_widget())
    });

    let (mut providers, child) = Theme::preprocess_before_compose(theme, child);
    let location = Location::stateful();
    providers.push(Provider::new(ShowingOverlays::default()));
    providers.push(Provider::writer(location.clone_writer(), None));
    providers.push(Provider::writer(Stateful::new(GlobalWidgets::default()), None));

    let mut root = FatObj::new(child);
    root
      .with_providers(providers)
      .on_disposed(move |_| location.write().release());
    let root = BuildCtx::get_mut().build(root.into_widget());

    self.root = root;
    self.dirty_marker().mark(root, DirtyPhase::Layout);
    root.on_mounted_subtree(self);
    root
  }

  pub(crate) fn root(&self) -> WidgetId { self.root }

  pub(crate) fn dummy_id(&self) -> WidgetId { self.dummy_id }

  pub(crate) fn dirty_marker(&self) -> DirtyMarker { DirtyMarker(self.dirty_set.clone()) }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self) {
    let wnd = self.window();
    let mut painter = wnd.painter.borrow_mut();
    let tree = wnd.tree();
    self.root().paint_subtree(tree, &mut painter);
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size, laid_out_queue: &mut Vec<WidgetId>) {
    loop {
      let Some((mut needs_layout, mut needs_paint, mut needs_position)) = self.layout_list() else {
        break;
      };
      let mut visual_roots = BTreeSet::new();
      while let Some(wid) = needs_layout.pop() {
        if wid.is_dropped(self) {
          continue;
        }
        if self.store.layout_box_size(wid).is_none() {
          needs_position.push(wid);
          let clamp = self
            .store
            .layout_info(wid)
            .map(|info| info.clamp)
            .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });

          let mut ctx = MeasureCtx::new(wid, self, laid_out_queue);
          let visual_rect = ctx.visual_box(wid);
          ctx.perform_layout(clamp);
          let new_rect = ctx.visual_box(wid);
          if visual_rect != new_rect
            && let Some(parent) = wid.parent(self)
          {
            let depth = parent.ancestors(self).count();
            visual_roots.insert((depth, parent));
          }
        }
      }

      // Handle position-only updates
      while let Some(wid) = needs_position.pop() {
        if wid.is_dropped(self) {
          continue;
        }
        laid_out_queue.push(wid);
        self.update_position_only(wid);
      }

      while let Some(wid) = needs_paint.pop() {
        if wid.is_dropped(self) {
          continue;
        }
        let depth = wid.ancestors(self).count();
        visual_roots.insert((depth, wid));
      }

      while let Some((depth, wid)) = visual_roots.pop_first() {
        let mut ctx = MeasureCtx::new(wid, self, laid_out_queue);
        let visual_rect = ctx.visual_box(wid);
        let new_rect = ctx.update_visual_box();
        if visual_rect != new_rect
          && let Some(parent) = wid.parent(self)
        {
          visual_roots.insert((depth - 1, parent));
        }
      }
    }
  }

  /// Update widget position only, without re-measuring.
  /// This is called for widgets marked with `DirtyPhase::Position`.
  fn update_position_only(&mut self, wid: WidgetId) {
    // The position of `wid` is determined by its parent.
    // So to update `wid`'s position, we need to call `place_children` on its
    // parent.
    if let Some(parent) = wid.parent(self) {
      self.call_place_children(parent);
    } else {
      // For root widget, just apply adjust_position
      let tree2 = unsafe { &*(self as *mut WidgetTree) };
      let mut provider_ctx = ProviderCtx::default();
      let mut layout_ctx = PlaceCtx { id: wid, tree: self, provider_ctx: &mut provider_ctx };
      let pos = wid
        .assert_get(tree2)
        .adjust_position(Point::zero(), &mut layout_ctx);
      self.store.layout_info_or_default(wid).pos = pos;
    }
  }

  /// Helper to call place_children on a widget with proper context setup.
  fn call_place_children(&mut self, wid: WidgetId) {
    let Some(size) = self.store.layout_box_size(wid) else {
      return;
    };

    let mut provider_ctx = if let Some(p) = wid.parent(self) {
      ProviderCtx::collect_from(p, self)
    } else {
      ProviderCtx::default()
    };

    let mut layout_ctx = PlaceCtx { id: wid, tree: self, provider_ctx: &mut provider_ctx };
    layout_ctx.perform_place(size);
  }

  pub(crate) fn alloc_node(&mut self, node: Box<dyn RenderQueryable>) -> WidgetId {
    new_node(&mut self.arena, node)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&LayoutInfo> {
    self.store.layout_info(id)
  }

  pub(crate) fn is_dirty(&self) -> bool { !self.dirty_set.borrow().is_empty() }

  #[allow(unused)]
  pub(crate) fn count(&self, wid: WidgetId) -> usize { wid.descendants(self).count() }

  pub(crate) fn window(&self) -> Rc<Window> {
    AppCtx::get_window(self.wnd_id).expect("Must initialize the widget tree before use it.")
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
  pub(crate) fn layout_list(&mut self) -> Option<(Vec<WidgetId>, Vec<WidgetId>, Vec<WidgetId>)> {
    if !self.is_dirty() {
      return None;
    }

    let mut needs_layout = vec![];
    let mut needs_paint = vec![];
    let mut needs_position = vec![];

    for (id, dirty) in self.dirty_set.borrow_mut().drain() {
      if id.is_dropped(self) {
        continue;
      }
      if dirty == DirtyPhase::Paint {
        needs_paint.push(id);
        continue;
      }
      if dirty == DirtyPhase::Position {
        needs_position.push(id);
        continue;
      }

      if dirty == DirtyPhase::LayoutSubtree {
        for w in id.0.descendants(&self.arena).map(WidgetId) {
          if let Some(info) = self.store.get_mut(&w) {
            info.size.take();
          }
        }
      } else if let Some(info) = self.store.get_mut(&id) {
        info.size.take();
      }

      let mut relayout_root = id;
      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        // The first one may be a pipe that is newly generated. Otherwise, if there
        // isn't layout information, it indicates that the ancestor marked for relayout
        // already.
        if self.store.layout_box_size(p).is_none() {
          break;
        }

        relayout_root = p;
        if let Some(info) = self.store.get_mut(&p) {
          info.size.take();
        }

        if !p.assert_get(self).size_affected_by_child() {
          break;
        }
      }
      needs_layout.push(relayout_root);
    }

    needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(self).count()));

    needs_position.retain(|w| self.store.layout_box_size(*w).is_some());

    Some((needs_layout, needs_paint, needs_position))
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

  pub(crate) fn set_detached_parent(&mut self, id: WidgetId, parent: Option<WidgetId>) {
    if let Some(parent) = parent {
      self.detached_parent.insert(id, parent);
    } else {
      self.detached_parent.remove(&id);
    }
  }

  pub(crate) fn detached_parent(&self, id: WidgetId) -> Option<WidgetId> {
    self.detached_parent.get(&id).copied()
  }

  pub(crate) fn clear_detached_parent(&mut self, id: WidgetId) { self.detached_parent.remove(&id); }

  pub(crate) fn remove_subtree(&mut self, id: WidgetId) {
    assert_ne!(id, self.root(), "You should detach the root widget before remove it.");

    id.0.descendants(&self.arena).for_each(|id| {
      let id = WidgetId(id);
      self.store.remove(id);
      self.detached_parent.remove(&id);
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

impl WidgetTree {
  pub fn new(wnd_id: WindowId) -> Self {
    let mut arena = TreeArena::new();
    let root = new_node(&mut arena, Box::new(PureRender(Void::default())));
    let dummy_id = new_node(&mut arena, Box::new(PureRender(Void::default())));
    dummy_id.0.remove(&mut arena);

    Self {
      root,
      dummy_id,
      wnd_id,
      arena,
      store: <_>::default(),
      detached_parent: <_>::default(),
      dirty_set: <_>::default(),
    }
  }

  pub fn disposed(&mut self) {
    let root = self.root();
    let dummy = new_node(&mut self.arena, Box::new(PureRender(Void::default())));
    root.insert_after(dummy, self);

    self.root = dummy;
    root.dispose_subtree(self);
  }
}

impl DirtyMarker {
  /// Mark the widget as dirty and return true if the widget was not already
  /// marked as dirty in this phase previously.
  pub(crate) fn mark(&self, id: WidgetId, scope: DirtyPhase) -> bool {
    let mut map = self.0.borrow_mut();
    if let Some(s) = map.get_mut(&id) {
      if scope.priority() > s.priority() {
        *s = scope;
        return true;
      }
      false
    } else {
      map.insert(id, scope).is_none()
    }
  }
}

#[declare(simple)]
#[derive(MultiChild)]
pub(crate) struct Root;

impl Render for Root {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let (ctx, children) = ctx.split_children();
    for c in children {
      ctx.layout_child(c, clamp);
    }

    clamp.max
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("root") }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::widget_layout_test;

  use super::*;
  #[cfg(target_arch = "wasm32")]
  use crate::test_helper::wasm_bindgen_test;
  use crate::{reset_test_env, test_helper::*};

  impl WidgetTree {
    pub(crate) fn content_root(&self) -> WidgetId { self.root.first_child(self).unwrap() }
  }

  fn empty_node(arena: &mut TreeArena) -> WidgetId {
    new_node(arena, Box::new(PureRender(Void::default())))
  }

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
        @MockBox { size: pipe!(*$read(no_boundary_size)) }
      }
    };
    let wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
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
        pipe!(*$read(parent)).map(move |p| fn_widget!{
          if p {
            @MockBox {
              size: Size::zero(),
              @ { pipe!($read(child).then(|| fn_widget!{ @Void {}})) }
            }.into_widget()
          } else {
            Void::default().into_widget()
          }
        })
      }
    };

    let wnd = TestWindow::from_widget(w);
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
    let w = fn_widget! { pipe!($read(trigger); fn_widget!{ @Void {}}) };

    let wnd = TestWindow::from_widget(w);
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

    let wnd = TestWindow::from_widget(fn_widget! {
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
    let mut queue = vec![];
    tree.layout(Size::new(512., 512.), &mut queue);
    assert_eq!(tree.count(tree.content_root()), 11);

    let root = tree.root();
    tree.dirty_marker().mark(root, DirtyPhase::Layout);
    let new_root = empty_node(&mut tree.arena);
    root.insert_after(new_root, tree);
    tree
      .dirty_marker()
      .mark(new_root, DirtyPhase::Layout);
    tree.detach(root);
    tree.remove_subtree(root);

    assert_eq!(tree.layout_list(), Some((vec![new_root], vec![], vec![])));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    reset_test_env!();

    let trigger = Stateful::new(1);
    let c_trigger = trigger.clone_writer();
    let widget = fn_widget! {
      @MockMulti {
        @pipe!{
          let size = if *$read(trigger) > 0 {
            Size::new(1., 1.)
          } else {
            Size::zero()
          };
          (0..3).map(move |_| @MockBox { size } )
        }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    let tree = wnd.tree_mut();
    let mut queue = vec![];
    tree.layout(Size::new(100., 100.), &mut queue);

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
    let wnd = TestWindow::new_with_size(w1, win_size);
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

    let wnd = TestWindow::from_widget(w2);
    wnd.draw_frame();
    let len_1_widget = wnd.painter.borrow_mut().finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }

  #[test]
  fn paint_phase_dirty() {
    reset_test_env!();

    #[derive(Default)]
    struct DirtyPaintOnly {
      paint_cnt: std::cell::Cell<usize>,
    }

    impl Render for DirtyPaintOnly {
      fn measure(&self, clamp: BoxClamp, _: &mut MeasureCtx) -> Size { clamp.max }

      fn paint(&self, _: &mut PaintingCtx) { self.paint_cnt.set(self.paint_cnt.get() + 1); }

      fn dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
    }

    let paint_cnt = Stateful::new(DirtyPaintOnly::default());
    let c_paint_cnt = paint_cnt.clone_writer();

    let (layout_cnt, w_layout_cnt) = split_value(0);

    let wnd = TestWindow::from_widget(fat_obj! {
      on_performed_layout: move |_| *$write(w_layout_cnt) += 1,
      @ { paint_cnt.clone_writer() }
    });

    wnd.draw_frame();

    assert_eq!(*layout_cnt.read(), 1);
    assert_eq!(c_paint_cnt.read().paint_cnt.get(), 1);

    {
      let _ = &mut *c_paint_cnt.write();
    }

    wnd.draw_frame();

    assert_eq!(*layout_cnt.read(), 1);
    assert_eq!(c_paint_cnt.read().paint_cnt.get(), 2);
  }

  #[derive(Declare, SingleChild)]
  pub struct FixedSizeBox {
    /// The specified size of the box.
    pub size: Size,
  }

  impl Render for FixedSizeBox {
    #[inline]
    fn measure(&self, _: BoxClamp, ctx: &mut MeasureCtx) -> Size {
      ctx.perform_single_child_layout(BoxClamp { min: self.size, max: self.size });
      self.size
    }

    #[inline]
    fn size_affected_by_child(&self) -> bool { true }
  }

  fn visual_overflow() -> GenWidget {
    fn_widget! {
      @MockMulti {
        @FixedSizeBox {
          size: Size::new(150., 50.),
          background: Color::GRAY,
          @MockStack {
            @ FixedSizeBox {
              size: Size::new(100., 100.),
              background: Color::GRAY,
              x: -30., y: 0.,
            }
            @ FixedSizeBox {
              size: Size::new(100., 100.),
              background: Color::GRAY,
              y: -20.,
            }
          }
        }
        @FixedSizeBox {
          size: Size::new(150., 50.),
          background: Color::GRAY,
          clip_boundary: true,
          @ FixedSizeBox {
            size: Size::new(100., 100.),
            background: Color::GRAY,
            x: -30., y: 20.,
          }
        }
      }
    }
    .r_into()
  }
  widget_layout_test!(
    visual_overflow,
    WidgetTester::new(visual_overflow()).with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0])
      .with_visual_rect(Rect::new(Point::new(-30., -20.), Size::new(180., 120.))),
    LayoutCase::new(&[0, 1]).with_visual_rect(Rect::new(Point::new(0., 0.), Size::new(150., 50.)))
  );
}
