use std::{
  cell::{Cell, RefCell},
  pin::Pin,
  rc::Rc,
  sync::{Arc, RwLock},
};

use crate::prelude::{
  widget_tree::{WidgetNode, WidgetTree},
  AsAttrs, BoxedWidget, BoxedWidgetInner, Event, EventCommon, StateAttr, WidgetId,
};
use crate::{animation::TickerProvider, prelude::widget_tree::WidgetChangeFlags};

use painter::{PaintCommand, Painter};
mod painting_context;
pub use painting_context::PaintingCtx;
mod event_context;
pub use event_context::EventCtx;
mod widget_context;
use ::text::shaper::TextShaper;
use text::{font_db::FontDB, TextReorder, TypographyStore};
pub use widget_context::*;
use winit::{event::ModifiersState, window::CursorIcon};
mod layout_context;
pub(crate) mod layout_store;
pub use layout_context::*;
pub use layout_store::BoxClamp;
pub(crate) use layout_store::LayoutStore;
mod build_context;
pub use build_context::BuildCtx;
mod generator_store;

pub(crate) struct Context {
  pub layout_store: LayoutStore,
  pub widget_tree: Pin<Box<WidgetTree>>,
  pub painter: Painter,
  pub modifiers: ModifiersState,
  pub cursor: Cell<Option<CursorIcon>>,
  pub font_db: Arc<RwLock<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  animation_ticker: Option<Rc<RefCell<Box<dyn TickerProvider>>>>,
  generator_store: generator_store::GeneratorStore,
}

impl Context {
  pub(crate) fn new(
    root: BoxedWidget,
    device_scale: f32,
    animation_ticker: Option<Box<dyn TickerProvider>>,
  ) -> Self {
    let ticker = animation_ticker.map(|ticker| Rc::new(RefCell::new(ticker)));
    let mut ctx = match root.0 {
      BoxedWidgetInner::Combination(c) => {
        let widget_tree = WidgetTree::new(WidgetNode::Combination(c));
        let mut ctx = Context::from_tree(widget_tree, device_scale, ticker);
        let tree = &ctx.widget_tree;
        let child = match tree.root().assert_get(tree) {
          WidgetNode::Combination(c) => {
            let mut build_ctx = BuildCtx::new(&ctx, tree.root());
            c.compose(&mut build_ctx)
          }
          WidgetNode::Render(_) => unreachable!(),
        };
        let root = tree.root();
        ctx.inflate_append(child, root);
        ctx
      }
      BoxedWidgetInner::Render(r) => {
        let widget_tree = WidgetTree::new(WidgetNode::Render(r));
        Context::from_tree(widget_tree, device_scale, ticker)
      }
      BoxedWidgetInner::SingleChild(s) => {
        let (rw, child) = s.unzip();
        let widget_tree = WidgetTree::new(WidgetNode::Render(rw));
        let mut ctx = Context::from_tree(widget_tree, device_scale, ticker);
        if let Some(child) = child {
          ctx.inflate_append(child, ctx.widget_tree.root());
        }
        ctx
      }
      BoxedWidgetInner::MultiChild(m) => {
        let (rw, children) = m.unzip();
        let widget_tree = WidgetTree::new(WidgetNode::Render(rw));
        let mut ctx = Context::from_tree(widget_tree, device_scale, ticker);
        let root = ctx.widget_tree.root();
        children
          .into_iter()
          .for_each(|w| ctx.inflate_append(w, root));
        ctx
      }
    };
    ctx.mark_layout_from_root();
    ctx
  }

  pub(crate) fn inflate_append(&mut self, widget: BoxedWidget, parent: WidgetId) {
    let mut stack = vec![(widget, parent)];

    while let Some((widget, p_wid)) = stack.pop() {
      match widget.0 {
        BoxedWidgetInner::Combination(c) => {
          let wid = p_wid.append_child(WidgetNode::Combination(c), self.widget_tree.as_mut());
          let mut ctx = BuildCtx::new(self, wid);
          let c = match wid.assert_get(&*self.widget_tree) {
            WidgetNode::Combination(c) => c,
            WidgetNode::Render(_) => unreachable!(),
          };
          let child = c.compose(&mut ctx);

          stack.push((child, wid));
        }
        BoxedWidgetInner::Render(rw) => {
          p_wid.append_child(WidgetNode::Render(rw), self.widget_tree.as_mut());
        }
        BoxedWidgetInner::SingleChild(s) => {
          let (rw, child) = s.unzip();
          let wid = p_wid.append_child(WidgetNode::Render(rw), self.widget_tree.as_mut());
          if let Some(child) = child {
            stack.push((child, wid));
          }
        }
        BoxedWidgetInner::MultiChild(m) => {
          let (rw, children) = m.unzip();
          let wid = p_wid.append_child(WidgetNode::Render(rw), self.widget_tree.as_mut());
          children
            .into_iter()
            .rev()
            .for_each(|w| stack.push((w, wid)));
        }
      };
    }
  }
  pub(crate) fn bubble_event<D, E, F, Attr>(
    &self,
    widget: WidgetId,
    default: F,
    mut dispatch: D,
  ) -> E
  where
    F: FnOnce(&Context, WidgetId) -> E,
    D: FnMut(&Attr, &mut E),
    E: Event + AsMut<EventCommon>,
    Attr: 'static,
  {
    let mut event = default(self, widget);
    for wid in widget.ancestors(&self.widget_tree) {
      if let Some(attr) = wid.assert_get(&self.widget_tree).find_attr::<Attr>() {
        event.as_mut().current_target = wid;
        dispatch(attr, &mut event);
        if event.bubbling_canceled() {
          break;
        }
      }
    }

    event
  }

  pub(crate) fn draw_tree(&mut self) -> Vec<PaintCommand> {
    let tree = &self.widget_tree;
    let mut wid = Some(tree.root());

    while let Some(mut id) = wid {
      if let Some(rect) = self.layout_store.layout_box_rect(id) {
        self.painter.save();
        self.painter.translate(rect.min_x(), rect.min_y());
      }
      let n = id.assert_get(&tree);
      if let WidgetNode::Render(ref r) = n {
        r.paint(&mut PaintingCtx {
          id,
          tree,
          layout_store: &self.layout_store,
          painter: &mut self.painter,
        })
      }

      // try to access child
      wid = id.first_child(tree);

      loop {
        if wid.is_some() {
          break;
        }
        // if child is none, try to access sibling
        if self.layout_store.layout_box_rect(id).is_some() {
          self.painter.restore()
        }
        wid = id.next_sibling(tree);

        // if there is no more sibling, parent subtree finished, try to access
        // parent sibling
        if wid.is_none() {
          match id.parent(tree) {
            Some(p) => id = p,
            None => break,
          }
        }
      }
    }

    self.painter.finish()
  }

  pub fn find_attr<A: 'static>(&self, widget: WidgetId) -> Option<&A> {
    widget.get(&self.widget_tree).and_then(AsAttrs::find_attr)
  }

  /// mark this id combination widget has changed
  fn mark_changed(&mut self, id: WidgetId) {
    let tree = &mut self.widget_tree;
    if let WidgetNode::Render(_) = id.assert_get(tree) {
      self.layout_store.mark_needs_layout(id, &self.widget_tree);
      // paint widget not effect widget size, it's detect by single child or
      // parent max limit.
    }
  }

  pub fn mark_layout_from_root(&mut self) {
    let tree = &self.widget_tree;
    if let Some(root) = tree.root().render_widget(tree) {
      self.layout_store.mark_needs_layout(root, tree);
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed.
  pub fn tree_repair(&mut self) {
    self
      .generator_store
      .update_dynamic_widgets(self.widget_tree.as_mut());
  }

  pub fn state_change_dispatch(&mut self) {
    while let Some((id, flag)) = self.widget_tree.pop_changed_widgets() {
      let attr = id
        .assert_get_mut(&mut self.widget_tree)
        .find_attr_mut::<StateAttr>();

      if flag.contains(WidgetChangeFlags::DIFFUSE) {
        if let Some(attr) = attr {
          attr.changed_notify()
        }
      }

      if flag.contains(WidgetChangeFlags::UNSILENT) {
        self.mark_changed(id);
      }
    }
  }

  #[allow(dead_code)]
  pub fn is_dirty(&self) -> bool {
    self.layout_store.is_dirty(&self.widget_tree) || self.generator_store.is_dirty()
  }

  pub fn descendants(&self) -> impl Iterator<Item = WidgetId> + '_ {
    self.widget_tree.root().descendants(&self.widget_tree)
  }

  pub(crate) fn from_tree(
    widget_tree: Pin<Box<WidgetTree>>,
    device_scale: f32,
    animation_ticker: Option<Rc<RefCell<Box<dyn TickerProvider>>>>,
  ) -> Self {
    let font_db = Arc::new(RwLock::new(FontDB::default()));
    let shaper = TextShaper::new(font_db.clone());
    let reorder = TextReorder::default();
    let typography_store = TypographyStore::new(reorder.clone(), font_db.clone(), shaper.clone());
    let painter = Painter::new(device_scale, typography_store.clone());
    Context {
      layout_store: <_>::default(),
      widget_tree,
      painter,
      cursor: <_>::default(),
      modifiers: <_>::default(),
      font_db: <_>::default(),
      shaper,
      reorder,
      typography_store,
      animation_ticker,
      generator_store: <_>::default(),
    }
  }

  pub(crate) fn trigger_animation_ticker(&mut self) -> bool {
    match &self.animation_ticker {
      Some(ticker) => ticker.borrow_mut().trigger(),
      None => false,
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use test::Bencher;

  use super::*;
  use crate::{
    prelude::BoxWidget,
    test::{embed_post::EmbedPost, key_embed_post::EmbedPostWithKey, recursive_row::RecursiveRow},
  };

  fn test_sample_create(width: usize, depth: usize) -> Context {
    let root = RecursiveRow { width, depth };
    Context::new(root.box_it(), 1., None)
  }

  #[test]
  fn drop_info_clear() {
    let post = EmbedPost::new(3);
    let mut ctx = Context::new(post.box_it(), 1., None);
    assert_eq!(ctx.widget_tree.count(), 20);
    ctx.mark_changed(ctx.widget_tree.root());
    ctx.drop(ctx.widget_tree.root());

    assert_eq!(ctx.is_dirty(), false);
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost::new(1000);
      Context::new(post.box_it(), 1., None);
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
    let mut ctx = Context::new(post.box_it(), 1., None);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair()
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(50, 2);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(100, 2);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 4);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 5);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[test]
  fn repair() {
    let mut ctx = test_sample_create(1, 1);
    ctx.mark_changed(ctx.widget_tree.root());
    assert!(!ctx.need_builds.is_empty());
    ctx.tree_repair();
    assert!(ctx.need_builds.is_empty());
  }
}
