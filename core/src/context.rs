use std::{
  sync::{Arc, RwLock},
  time::Instant,
};

use crate::{
  prelude::{AnimationCtrl, AnimationId},
  ticker::FrameTicker,
};
use crate::{
  prelude::{AnimationStore, WidgetId},
  ticker::FrameMsg,
};

mod painting_context;
pub use painting_context::PaintingCtx;
mod event_context;
pub use event_context::EventCtx;
mod widget_context;
use ::text::shaper::TextShaper;
use text::{font_db::FontDB, TextReorder, TypographyStore};
pub use widget_context::*;
mod layout_context;
pub(crate) mod layout_store;
pub use layout_context::*;
pub use layout_store::BoxClamp;
pub(crate) use layout_store::LayoutStore;
pub(crate) mod build_context;
pub use build_context::BuildCtx;

pub(crate) mod generator_store;

pub(crate) struct Context {
  pub layout_store: LayoutStore,
  pub font_db: Arc<RwLock<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  pub generator_store: generator_store::GeneratorStore,
  pub frame_ticker: FrameTicker,
  pub animations_store: AnimationStore,
}

impl Context {
  pub fn expr_widgets_dirty(&self) -> bool { self.generator_store.is_dirty() }

  pub fn on_widget_drop(&mut self, id: WidgetId) {
    self.generator_store.on_widget_drop(id);
    self.layout_store.remove(id);
  }

  pub fn begin_frame(&mut self) { self.frame_ticker.emit(FrameMsg::Ready(Instant::now())); }

  pub fn end_frame(&mut self) {
    // todo: frame cache is not a good choice? because not every text will relayout
    // in every frame.
    self.shaper.end_frame();
    self.reorder.end_frame();
    self.typography_store.end_frame();

    self.frame_ticker.emit(FrameMsg::Finish);
  }

  pub fn register_animate(&mut self, animate: Box<dyn AnimationCtrl>) -> AnimationId {
    self.animations_store.register(animate)
  }
}

impl Default for Context {
  fn default() -> Self {
    let mut font_db = FontDB::default();
    font_db.load_system_fonts();
    let font_db = Arc::new(RwLock::new(font_db));
    let shaper = TextShaper::new(font_db.clone());
    let reorder = TextReorder::default();
    let typography_store = TypographyStore::new(reorder.clone(), font_db.clone(), shaper.clone());
    let generator_store = generator_store::GeneratorStore::default();
    let frame_ticker = FrameTicker::default();
    let animations_store = AnimationStore::new(frame_ticker.frame_tick_stream());
    Context {
      layout_store: <_>::default(),
      font_db: <_>::default(),
      shaper,
      reorder,
      typography_store,
      generator_store,
      frame_ticker,
      animations_store,
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use test::Bencher;

  use super::*;
  use crate::{
    prelude::IntoWidget,
    test::{embed_post::EmbedPost, key_embed_post::EmbedPostWithKey, recursive_row::RecursiveRow},
  };

  fn test_sample_create(width: usize, depth: usize) -> Context {
    let root = RecursiveRow { width, depth };
    Context::new(root.into_widget(), 1.)
  }

  #[test]
  fn drop_info_clear() {
    let post = EmbedPost::new(3);
    let mut ctx = Context::new(post.into_widget(), 1.);
    assert_eq!(ctx.widget_tree.count(), 17);
    ctx.mark_root_dirty();
    ctx.drop_subtree(ctx.widget_tree.root());

    assert_eq!(ctx.is_dirty(), false);
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost::new(1000);
      Context::new(post.into_widget(), 1.);
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
    let mut ctx = Context::new(post.into_widget(), 1.);
    b.iter(|| {
      ctx.mark_root_dirty();
      ctx.tree_repair()
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(50, 2);
    b.iter(|| {
      ctx.mark_root_dirty();
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(100, 2);
    b.iter(|| {
      ctx.mark_root_dirty();
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 4);
    b.iter(|| {
      ctx.mark_root_dirty();
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 5);
    b.iter(|| {
      ctx.mark_root_dirty();
      ctx.tree_repair();
    })
  }
}
