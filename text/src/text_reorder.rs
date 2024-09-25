use std::ops::Range;

use ribir_algo::{FrameCache, Sc, Substr};
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
#[derive(Default)]
pub struct TextReorder {
  cache: FrameCache<Substr, Sc<ReorderResult>>,
}

impl TextReorder {
  pub fn get_cache(&mut self, text: &Substr) -> Option<Sc<ReorderResult>> {
    self.cache.get(text).cloned()
  }

  pub fn reorder_text(&mut self, text: &Substr) -> &Sc<ReorderResult> {
    self.cache.get_or_insert(text.clone(), || {
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

      Sc::new(ReorderResult { original_classes: info.original_classes, paras })
    })
  }

  pub fn end_frame(&mut self) { self.cache.end_frame("Text Reorder"); }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let mut reorder = TextReorder::default();
    let text: Substr = concat!["א", "ב", "ג", "a", "b", "c",].into();
    // No cache exists
    assert!(reorder.get_cache(&text).is_none());

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

    assert!(reorder.get_cache(&text).is_some());

    reorder.end_frame();
    reorder.end_frame();
    assert!(reorder.get_cache(&text).is_none());
  }
}
