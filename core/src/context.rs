use std::{
  sync::{Arc, RwLock},
  time::Instant,
};

use crate::ticker::FrameMsg;
use crate::ticker::FrameTicker;

mod lifecycle_context;
pub use lifecycle_context::LifeCycleCtx;
mod painting_context;
pub use painting_context::PaintingCtx;
mod event_context;
pub use event_context::EventCtx;
mod widget_context;
use ::text::shaper::TextShaper;
mod layout_context;
pub use layout_context::*;
use text::{font_db::FontDB, TextReorder, TypographyStore};
pub use widget_context::*;
pub(crate) mod build_context;
pub use build_context::BuildCtx;

pub struct AppContext {
  pub font_db: Arc<RwLock<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  pub frame_ticker: FrameTicker,
}

impl AppContext {
  pub fn begin_frame(&mut self) { self.frame_ticker.emit(FrameMsg::Ready(Instant::now())); }

  pub fn end_frame(&mut self) {
    // todo: frame cache is not a good choice? because not every text will relayout
    // in every frame.
    self.shaper.end_frame();
    self.reorder.end_frame();
    self.typography_store.end_frame();

    self.frame_ticker.emit(FrameMsg::Finish);
  }
}

impl Default for AppContext {
  fn default() -> Self {
    let mut font_db = FontDB::default();
    font_db.load_system_fonts();
    let font_db = Arc::new(RwLock::new(font_db));
    let shaper = TextShaper::new(font_db.clone());
    let reorder = TextReorder::default();
    let typography_store = TypographyStore::new(reorder.clone(), font_db.clone(), shaper.clone());
    let frame_ticker = FrameTicker::default();

    AppContext {
      font_db: <_>::default(),
      shaper,
      reorder,
      typography_store,
      frame_ticker,
    }
  }
}
