use std::{
  ops::Range,
  sync::{Arc, RwLock},
};

use ribir_algo::{FrameCache, Substr};
use unicode_bidi::{BidiClass, BidiInfo, Level, LevelRun};

pub struct Paragraph {
  pub levels: Vec<Level>,
  pub runs: Vec<LevelRun>,
  pub range: Range<usize>,
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
    self.cache.write().unwrap().get(text).cloned()
  }

  pub fn reorder_text(&self, text: &Substr) -> Arc<ReorderResult> {
    self.get_from_cache(text).unwrap_or_else(|| {
      let info = BidiInfo::new(text, None);
      let mut paras: Vec<Paragraph> = info
        .paragraphs
        .iter()
        .map(|p| {
          let (levels, runs) = info.visual_runs(p, p.range.clone());
          Paragraph { levels, runs, range: p.range.clone() }
        })
        .collect();

      if paras.is_empty() || text.ends_with('\r') || text.ends_with('\n') {
        paras.push(Paragraph {
          levels: vec![],
          runs: vec![Range { start: text.len(), end: text.len() }],
          range: Range { start: text.len(), end: text.len() },
        })
      }

      let result = Arc::new(ReorderResult { original_classes: info.original_classes, paras });
      let mut cache = self.cache.write().unwrap();
      cache.put(text.clone(), result.clone());
      result
    })
  }

  pub fn end_frame(&mut self) {
    self
      .cache
      .write()
      .unwrap()
      .end_frame("Text Reorder");
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let mut reorder = TextReorder::default();
    let text: Substr = concat!["א", "ב", "ג", "a", "b", "c",].into();
    // No cache exists
    assert!(reorder.get_from_cache(&text).is_none());

    let result = reorder.reorder_text(&text);
    assert_eq!(result.paras.len(), 1);

    let Paragraph { runs, levels, .. } = &result.paras[0];

    assert_eq!(
      &levels
        .iter()
        .map(|l| l.number())
        .collect::<Vec<_>>(),
      &[1, 1, 1, 1, 1, 1, 2, 2, 2]
    );

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0], 6..9);
    assert_eq!(runs[1], 0..6);

    assert!(reorder.get_from_cache(&text).is_some());

    reorder.end_frame();
    reorder.end_frame();
    assert!(reorder.get_from_cache(&text).is_none());
  }
}
