use algo::FrameCache;
use arcstr::Substr;
use std::sync::{Arc, RwLock};
use unicode_bidi::{BidiClass, BidiInfo, Level, LevelRun};

pub struct Paragraph {
  pub levels: Vec<Level>,
  pub runs: Vec<LevelRun>,
}
pub struct ReorderResult {
  pub original_classes: Vec<BidiClass>,

  pub paras: Vec<Paragraph>,
}

// unnecessary cache
#[derive(Clone, Default)]
pub struct TextReorder {
  cache: Arc<RwLock<FrameCache<Substr, Arc<ReorderResult>>>>,
}

impl TextReorder {
  pub fn get_from_cache(&self, text: &Substr) -> Option<Arc<ReorderResult>> {
    self.cache.read().unwrap().get(text).cloned()
  }

  pub fn reorder_text(&self, text: &Substr) -> Arc<ReorderResult> {
    self.get_from_cache(text).unwrap_or_else(|| {
      let info = BidiInfo::new(text, None);
      let paras = info
        .paragraphs
        .iter()
        .map(|p| {
          let (levels, runs) = info.visual_runs(p, p.range.clone());
          Paragraph { levels, runs }
        })
        .collect();

      let result = Arc::new(ReorderResult {
        original_classes: info.original_classes,
        paras,
      });
      let mut cache = self.cache.write().unwrap();
      cache.insert(text.clone(), result.clone());
      result
    })
  }

  pub fn end_frame(&mut self) { self.cache.write().unwrap().end_frame("Text Reorder") }
}
