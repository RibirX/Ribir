use std::{
  cell::RefCell,
  rc::Rc,
  sync::{Arc, RwLock},
};

use crate::{
  builtin_widgets::{FullTheme, InheritTheme, Theme},
  clipboard::{Clipboard, UnSpportClipboard},
};

pub use futures::task::SpawnError;
use futures::{
  executor::{block_on, LocalPool},
  task::LocalSpawnExt,
  Future,
};
use ribir_text::shaper::TextShaper;
use ribir_text::{font_db::FontDB, TextReorder, TypographyStore};

#[derive(Clone)]
pub struct AppContext {
  pub app_theme: Rc<Theme>,
  pub font_db: Arc<RwLock<FontDB>>,
  pub shaper: TextShaper,
  pub reorder: TextReorder,
  pub typography_store: TypographyStore,
  pub executor: Executor,
  pub clipboard: Rc<RefCell<dyn Clipboard>>,
}

#[derive(Clone)]
pub struct Executor {
  #[cfg(feature = "thread-pool")]
  pub thread_pool: futures::executor::ThreadPool,
  pub local: Rc<RefCell<LocalPool>>,
}

impl Default for Executor {
  fn default() -> Self {
    Self {
      #[cfg(feature = "thread-pool")]
      thread_pool: futures::executor::ThreadPool::new().unwrap(),
      local: Rc::new(RefCell::new(LocalPool::default())),
    }
  }
}

impl AppContext {
  pub(crate) fn end_frame(&mut self) {
    // todo: frame cache is not a good choice? because not every text will relayout
    // in every frame.
    self.shaper.end_frame();
    self.reorder.end_frame();
    self.typography_store.end_frame();
  }

  pub fn load_font_from_theme(&self, theme: Rc<Theme>) {
    let mut font_db = self.font_db.write().unwrap();
    match &*theme {
      Theme::Full(FullTheme { font_bytes, font_files, .. })
      | Theme::Inherit(InheritTheme { font_bytes, font_files, .. }) => {
        if let Some(font_bytes) = font_bytes {
          font_bytes
            .iter()
            .for_each(|data| font_db.load_from_bytes(data.clone()));
        }
        if let Some(font_files) = font_files {
          font_files.iter().for_each(|path| {
            let _ = font_db.load_font_file(path);
          });
        }
      }
    }
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

    AppContext {
      font_db,
      app_theme: <_>::default(),
      shaper,
      reorder,
      typography_store,
      executor: <_>::default(),
      clipboard: Rc::new(RefCell::new(UnSpportClipboard {})),
    }
  }
}

impl Executor {
  #[inline]
  pub fn spawn_local<Fut>(&self, future: Fut) -> Result<(), SpawnError>
  where
    Fut: Future<Output = ()> + 'static,
  {
    self.local.borrow().spawner().spawn_local(future)
  }

  #[cfg(feature = "thread-pool")]
  #[inline]
  pub fn spawn_in_pool<Fut>(&self, future: Fut)
  where
    Fut: 'static + Future<Output = ()> + Send,
  {
    self.thread_pool.spawn_ok(future)
  }
}

impl AppContext {
  pub fn wait_future<F: Future>(f: F) -> F::Output { block_on(f) }
}
