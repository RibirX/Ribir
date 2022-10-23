use std::{
  rc::Rc,
  sync::{Arc, RwLock},
  time::Instant,
};

use crate::{builtin_widgets::material, ticker::FrameTicker};
use crate::{builtin_widgets::Theme, ticker::FrameMsg};

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
mod tree_context;
pub use tree_context::*;

#[derive(Clone)]
pub struct AppContext {
  pub app_theme: Rc<Theme>,
  pub font_db: Arc<RwLock<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  pub frame_ticker: FrameTicker,
}

impl AppContext {
  pub fn begin_frame(&mut self) { self.frame_ticker.emit(FrameMsg::NewFrame(Instant::now())); }
  pub fn layout_ready(&mut self) {
    self
      .frame_ticker
      .emit(FrameMsg::LayoutReady(Instant::now()));
  }

  pub fn end_frame(&mut self) {
    // todo: frame cache is not a good choice? because not every text will relayout
    // in every frame.
    self.shaper.end_frame();
    self.reorder.end_frame();
    self.typography_store.end_frame();

    self.frame_ticker.emit(FrameMsg::Finish(Instant::now()));
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
      app_theme: Rc::new(material::purple::light()),
      shaper,
      reorder,
      typography_store,
      frame_ticker,
    }
  }
}
