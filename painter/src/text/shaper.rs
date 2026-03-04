use std::{
  cell::RefCell,
  hash::{Hash, Hasher},
};

use ribir_algo::{FrameCache, Rc, Substr};
use swash::shape::ShapeContext;

use super::{GlyphUnit, font_db::GlyphBaseline};
use crate::{
  Glyph, TextDirection,
  font_db::{Face, FontDB, ID},
  text::GlyphId,
};

pub const NEWLINE_GLYPH_ID: GlyphId = GlyphId(u16::MAX);
/// Shaper to shape the `text` using provided font faces, and will do BIDI
/// reordering before to shape text.
///
/// This shaper will cache shaper result for per frame.
pub struct TextShaper {
  font_db: Rc<RefCell<FontDB>>,
  shape_cache: FrameCache<ShapeKey, Rc<ShapeResult>>,
}

#[derive(Debug, Clone)]
pub struct ShapeResult {
  pub text: Substr,
  pub glyphs: Vec<Glyph>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct ShapeKey {
  face_ids: Box<[ID]>,
  text: Substr,
  direction: TextDirection,
  baseline: GlyphBaseline,
}

struct GlyphsWithoutFallback {
  glyphs: Vec<Glyph>,
}

impl TextShaper {
  #[inline]
  pub fn new(font_db: Rc<RefCell<FontDB>>) -> Self { Self { font_db, shape_cache: <_>::default() } }

  pub fn end_frame(&mut self) { self.shape_cache.end_frame("Text shape"); }

  /// Shape text and return the glyphs, caller should do text reorder before
  /// call this method.
  pub fn shape_text(
    &mut self, text: &Substr, face_ids: &[ID], direction: TextDirection, baseline: GlyphBaseline,
  ) -> Rc<ShapeResult> {
    if let Some(res) = self.get_cache(text, face_ids, direction, baseline) {
      res.clone()
    } else {
      let mut glyphs = self
        .shape_text_with_fallback(text, direction, face_ids, baseline)
        .unwrap_or_default();

      if let Some(last_char) = text.bytes().last()
        && (last_char == b'\r' || last_char == b'\n')
      {
        // Check if the last glyph already corresponds to the newline character.
        // swash may not emit a glyph for control characters, so we may need to
        // append a synthetic one.
        let newline_cluster = text.len().saturating_sub(1) as u32;
        if let Some(g) = glyphs.last_mut()
          && g.cluster == newline_cluster
        {
          g.glyph_id = NEWLINE_GLYPH_ID;
        } else {
          // swash didn't emit a glyph for the newline — append a synthetic one
          let face_id = glyphs
            .last()
            .map(|g| g.face_id)
            .unwrap_or_default();
          glyphs.push(Glyph {
            face_id,
            glyph_id: NEWLINE_GLYPH_ID,
            x_advance: GlyphUnit::ZERO,
            y_advance: GlyphUnit::ZERO,
            x_offset: GlyphUnit::ZERO,
            y_offset: GlyphUnit::ZERO,
            cluster: newline_cluster,
          });
        }
      }

      let glyphs = Rc::new(ShapeResult { text: text.clone(), glyphs });
      self.shape_cache.put(
        ShapeKey { face_ids: face_ids.into(), text: text.clone(), direction, baseline },
        glyphs.clone(),
      );
      glyphs
    }
  }

  /// Directly shape text without bidi reordering.
  pub fn shape_text_with_fallback(
    &self, text: &str, dir: TextDirection, face_ids: &[ID], baseline: GlyphBaseline,
  ) -> Option<Vec<Glyph>> {
    let mut font_fallback = FallBackFaceHelper::new(face_ids, &self.font_db);
    let face = font_fallback.next_fallback_face(text)?;

    let GlyphsWithoutFallback { mut glyphs } = Self::directly_shape(text, baseline, dir, &face);
    let mut new_part = vec![(0, glyphs.len(), font_fallback.clone())];
    loop {
      if new_part.is_empty() {
        break;
      }
      let miss_part = collect_miss_part(&glyphs, &new_part);
      new_part = regen_miss_part(text, dir, baseline, &mut glyphs, miss_part);
    }

    Some(glyphs)
  }

  fn directly_shape(
    text: &str, baseline: GlyphBaseline, dir: TextDirection, face: &Face,
  ) -> GlyphsWithoutFallback {
    let mut context = ShapeContext::new();
    let swash_font = face.as_font_ref();
    let mut shaper = context
      .builder(swash_font)
      .direction(dir.into())
      .size(face.units_per_em() as f32)
      .features([("rlig", 1), ("liga", 1), ("clig", 1)])
      .build();
    shaper.add_str(text);

    let mut glyphs = Vec::new();
    let shift = face.baseline_offset(baseline);
    let scale = GlyphUnit::UNITS_PER_EM as f32 / face.units_per_em() as f32;
    let shift = GlyphUnit::new(f32::ceil(shift as f32 * scale) as i32);

    shaper.shape_with(|cluster| {
      for g in cluster.glyphs {
        let mut glyph = Glyph::new(
          GlyphId(g.id),
          cluster.source.start,
          g.advance as i32,
          0,
          g.x as i32,
          g.y as i32,
          face,
        );
        if dir.is_horizontal() {
          glyph.y_offset -= shift;
        } else {
          glyph.x_offset -= shift;
        }
        glyphs.push(glyph);
      }
    });

    if matches!(dir, TextDirection::RightToLeft | TextDirection::BottomToTop) {
      glyphs.reverse();
    }

    GlyphsWithoutFallback { glyphs }
  }

  pub fn get_cache(
    &mut self, text: &str, face_ids: &[ID], direction: TextDirection, baseline: GlyphBaseline,
  ) -> Option<Rc<ShapeResult>> {
    self
      .shape_cache
      .get(&(face_ids, text, direction, baseline) as &dyn ShapeKeySlice)
      .cloned()
  }

  pub fn font_db(&self) -> &Rc<RefCell<FontDB>> { &self.font_db }
}

fn collect_miss_part<'a>(
  glyphs: &[Glyph], new_part: &[(usize, usize, FallBackFaceHelper<'a>)],
) -> Vec<(usize, usize, FallBackFaceHelper<'a>)> {
  let mut miss_parts = vec![];
  for (start, end, helper) in new_part {
    let mut miss_start = None;
    let mut last_miss_cluster = None;
    glyphs[*start..*end]
      .iter()
      .enumerate()
      .map(|(idx, glyph)| (idx + *start, glyph))
      .for_each(|(idx, glyph)| {
        if glyph.is_miss() {
          if miss_start.is_none() {
            miss_start = Some(idx);
          }
          last_miss_cluster = Some(glyph.cluster);
        } else if last_miss_cluster
          .as_ref()
          .is_none_or(|cluster| *cluster != glyph.cluster)
          && miss_start.is_some()
        {
          miss_parts.push((miss_start.take().unwrap(), idx, helper.clone()));
        }
      });
    if miss_start.is_some() {
      miss_parts.push((miss_start.take().unwrap(), *end, helper.clone()));
    }
  }

  miss_parts.iter_mut().for_each(|(start, _, _)| {
    while 0 < *start && glyphs[*start - 1].cluster == glyphs[*start].cluster {
      *start -= 1;
    }
  });

  miss_parts
}

fn regen_miss_part<'a>(
  text: &str, dir: TextDirection, baseline: GlyphBaseline, glyphs: &mut Vec<Glyph>,
  miss_part: Vec<(usize, usize, FallBackFaceHelper<'a>)>,
) -> Vec<(usize, usize, FallBackFaceHelper<'a>)> {
  let is_rtl = matches!(dir, TextDirection::RightToLeft | TextDirection::BottomToTop);

  let cluster_to_range_byte = |glyphs: &Vec<Glyph>, idx: usize| -> usize {
    let is_end = (is_rtl && 0 == idx) || (!is_rtl && idx == glyphs.len());
    match (is_end, is_rtl) {
      (true, _) => text.len(),
      (false, true) => glyphs[idx - 1].cluster as usize,
      (false, false) => glyphs[idx].cluster as usize,
    }
  };

  let mut offset = 0_i32;
  let mut new_part = vec![];
  for (mut miss_start, mut miss_end, mut helper) in miss_part.into_iter() {
    miss_start = ((miss_start as i32) + offset) as usize;
    miss_end = ((miss_end as i32) + offset) as usize;
    let start_byte = cluster_to_range_byte(glyphs, miss_start);
    let end_byte = cluster_to_range_byte(glyphs, miss_end);
    let miss_range = match is_rtl {
      true => end_byte..start_byte,
      false => start_byte..end_byte,
    };
    let miss_text = &text[miss_range.clone()];
    if let Some(face) = helper.next_fallback_face(miss_text) {
      let res = TextShaper::directly_shape(miss_text, baseline, dir, &face);
      let mut res_glyphs = res.glyphs;
      for g in res_glyphs.iter_mut() {
        g.cluster += miss_range.start as u32;
      }

      offset += (res_glyphs.len() as i32) - ((miss_end - miss_start) as i32);
      new_part.push((miss_start, miss_start + res_glyphs.len(), helper));
      glyphs.splice(miss_start..miss_end, res_glyphs);
    }
  }
  new_part
}

trait ShapeKeySlice {
  fn face_ids(&self) -> &[ID];
  fn text(&self) -> &str;
  fn direction(&self) -> TextDirection;
  fn baseline(&self) -> GlyphBaseline;
}

impl<'a> std::borrow::Borrow<dyn ShapeKeySlice + 'a> for ShapeKey {
  fn borrow(&self) -> &(dyn ShapeKeySlice + 'a) { self }
}

impl Hash for dyn ShapeKeySlice + '_ {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.face_ids().hash(state);
    self.text().hash(state);
    self.direction().hash(state);
    self.baseline().hash(state);
  }
}

impl PartialEq for dyn ShapeKeySlice + '_ {
  fn eq(&self, other: &Self) -> bool {
    self.face_ids() == other.face_ids()
      && self.text() == other.text()
      && self.direction() == other.direction()
      && self.baseline() == other.baseline()
  }
}

impl Eq for dyn ShapeKeySlice + '_ {}

impl ShapeKeySlice for ShapeKey {
  fn face_ids(&self) -> &[ID] { &self.face_ids }

  fn text(&self) -> &str { &self.text }

  fn direction(&self) -> TextDirection { self.direction }

  fn baseline(&self) -> GlyphBaseline { self.baseline }
}

impl ShapeKeySlice for (&[ID], &str, TextDirection, GlyphBaseline) {
  fn face_ids(&self) -> &[ID] { self.0 }

  fn text(&self) -> &str { self.1 }

  fn direction(&self) -> TextDirection { self.2 }

  fn baseline(&self) -> GlyphBaseline { self.3 }
}

impl From<TextDirection> for swash::shape::Direction {
  fn from(dir: TextDirection) -> Self {
    match dir {
      TextDirection::LeftToRight => swash::shape::Direction::LeftToRight,
      TextDirection::RightToLeft => swash::shape::Direction::RightToLeft,
      TextDirection::TopToBottom => swash::shape::Direction::LeftToRight,
      TextDirection::BottomToTop => swash::shape::Direction::RightToLeft,
    }
  }
}

#[derive(Clone)]
struct FallBackFaceHelper<'a> {
  ids: Vec<ID>,
  font_db: &'a RefCell<FontDB>,
  face_idx: usize,
}

impl<'a> FallBackFaceHelper<'a> {
  fn new(ids: &'a [ID], font_db: &'a RefCell<FontDB>) -> Self {
    let mut ids = ids.to_vec();
    let set: ahash::HashSet<ID> = ahash::HashSet::from_iter(ids.iter().cloned());

    {
      let font_db = font_db.borrow();
      let default_ids = font_db.default_fonts();
      for id in default_ids.iter() {
        if set.contains(id) {
          continue;
        }
        ids.push(*id);
      }
    }

    Self { ids, font_db, face_idx: 0 }
  }

  fn next_fallback_face(&mut self, text: &str) -> Option<Face> {
    let mut font_db = self.font_db.borrow_mut();
    loop {
      if self.face_idx >= self.ids.len() {
        return None;
      }

      let face = self
        .ids
        .get(self.face_idx)
        .and_then(|id| font_db.face_data_or_insert(*id))
        .cloned();

      self.face_idx += 1;
      if self.face_idx == self.ids.len() {
        return face;
      } else {
        let face = face.filter(|f| match text.is_empty() {
          true => true,
          false => text.chars().any(|c| f.has_char(c)),
        });
        if face.is_some() {
          return face;
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{FontFace, FontFamily, TextDirection};

  #[test]
  fn smoke() {
    let mut shaper = TextShaper::new(<_>::default());
    shaper.font_db.borrow_mut().load_system_fonts();

    let text: Substr = concat!["א", "ב", "ג", "a", "b", "c",].into();
    let ids = shaper
      .font_db
      .borrow_mut()
      .select_all_match(&FontFace {
        families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
        ..<_>::default()
      });
    let dir = TextDirection::LeftToRight;
    let baseline = GlyphBaseline::Alphabetic;
    // No cache exists
    assert!(
      shaper
        .get_cache(&text, &ids, dir, baseline)
        .is_none()
    );

    let result = shaper.shape_text(&text, &ids, dir, GlyphBaseline::Alphabetic);
    assert_eq!(result.glyphs.len(), 6);

    assert!(
      shaper
        .get_cache(&text, &ids, dir, baseline)
        .is_some()
    );

    shaper.end_frame();
    shaper.end_frame();
    assert!(
      shaper
        .get_cache(&text, &ids, dir, baseline)
        .is_none()
    );
  }

  #[test]
  fn font_fallback() {
    let mut shaper = TextShaper::new(<_>::default());
    let path = env!("CARGO_MANIFEST_DIR").to_owned();
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path.clone() + "/../fonts/DejaVuSans.ttf");
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path + "/../fonts/NotoSerifSC-Bold.你好世界.otf");

    let ids_latin = shaper
      .font_db
      .borrow_mut()
      .select_all_match(&FontFace {
        families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
        ..<_>::default()
      });

    let ids_all = shaper
      .font_db
      .borrow_mut()
      .select_all_match(&FontFace {
        families: Box::new([
          FontFamily::Name("DejaVu Sans".into()),
          FontFamily::Name("Noto Serif SC".into()),
        ]),
        ..<_>::default()
      });

    let dir = TextDirection::LeftToRight;
    let latin1 = shaper.shape_text(
      &"hello world! 你好，世界".into(),
      &ids_latin,
      dir,
      GlyphBaseline::Alphabetic,
    );
    assert_eq!(
      latin1
        .glyphs
        .iter()
        .fold((0_u32, 0_u32), |(mut latin, mut chinese), g| {
          if g.is_not_miss() {
            latin += 1;
          } else {
            chinese += 1
          }
          (latin, chinese)
        }),
      (13, 5)
    );

    let fallback_chinese = shaper.shape_text(
      &"hello world! 你好，世界".into(),
      &ids_all,
      dir,
      GlyphBaseline::Alphabetic,
    );
    let clusters = fallback_chinese
      .glyphs
      .iter()
      .map(|g| g.cluster)
      .collect::<Vec<_>>();
    assert!(
      fallback_chinese
        .glyphs
        .iter()
        .all(|glyph| glyph.is_not_miss())
    );
    assert_eq!(&clusters, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 19, 22, 25]);
  }

  #[test]
  fn shape_miss_font() {
    let mut shaper = TextShaper::new(<_>::default());

    let dir = TextDirection::LeftToRight;
    let result = shaper.shape_text(&"你好世界".into(), &[], dir, GlyphBaseline::Alphabetic);
    assert_eq!(result.glyphs.len(), 4);
  }

  #[test]
  fn partiall_glyphs() {
    let font_db = Rc::new(RefCell::new(FontDB::default()));
    let _ = font_db
      .borrow_mut()
      .load_font_file(env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/GaramondNo8-Reg.ttf");
    let _ = font_db.borrow_mut().load_font_file(
      env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/Nunito-VariableFont_wght.ttf",
    );
    let mut shaper = TextShaper::new(font_db.clone());

    let text: Substr = "р҈р҈р҈р҈".into();

    {
      let ids = shaper
        .font_db
        .borrow_mut()
        .select_all_match(&FontFace {
          families: Box::new([
            FontFamily::Name("GaramondNo8".into()),
            FontFamily::Name("Nunito".into()),
          ]),
          ..<_>::default()
        });
      let res = shaper.shape_text(
        &text.substr(..),
        &ids,
        TextDirection::LeftToRight,
        GlyphBaseline::Alphabetic,
      );
      assert_eq!(res.glyphs.len(), 8);
      assert!(res.glyphs.iter().all(|glyph| glyph.is_miss()));
    }

    {
      let _ = font_db
        .borrow_mut()
        .load_font_file(env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf");
      let ids = shaper
        .font_db
        .borrow_mut()
        .select_all_match(&FontFace {
          families: Box::new([
            FontFamily::Name("GaramondNo8".into()),
            FontFamily::Name("Nunito".into()),
            FontFamily::Name("DejaVu Sans".into()),
          ]),
          ..<_>::default()
        });
      shaper.shape_cache.clear();
      let res = shaper.shape_text(
        &text.substr(..),
        &ids,
        TextDirection::LeftToRight,
        GlyphBaseline::Alphabetic,
      );
      assert!(res.glyphs.len() == 8);
      assert!(res.glyphs.iter().all(|glyph| glyph.is_not_miss()));
    }
  }

  #[test]
  fn shape_compose_emoji() {
    let mut shaper = TextShaper::new(<_>::default());
    let path = env!("CARGO_MANIFEST_DIR").to_owned();
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path.clone() + "/../fonts/DejaVuSans.ttf");
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path + "/../fonts/NotoSerifSC-Bold.你好世界.otf");
    let ids_all = shaper
      .font_db
      .borrow_mut()
      .select_all_match(&FontFace {
        families: Box::new([
          FontFamily::Name("DejaVu Sans".into()),
          FontFamily::Name("Noto Serif SC".into()),
        ]),
        ..<_>::default()
      });

    let res = shaper.shape_text(
      &"👨‍👩‍👦‍👦".into(),
      &ids_all,
      TextDirection::LeftToRight,
      GlyphBaseline::Alphabetic,
    );
    assert_eq!(res.glyphs.len(), 4);
  }

  #[test]
  fn material_search_ligature_should_shape_to_single_glyph() {
    let mut shaper = TextShaper::new(<_>::default());
    let ids = {
      let mut db = shaper.font_db.borrow_mut();
      db.load_from_bytes(include_bytes!("../../../fonts/material-search.ttf").to_vec());
      db.select_all_match(&FontFace {
        families: Box::new([FontFamily::Name("Material Symbols Rounded 48pt".into())]),
        ..<_>::default()
      })
    };

    assert!(!ids.is_empty(), "material search font family not matched");

    let shaped = shaper.shape_text(
      &Substr::from("search"),
      &ids,
      TextDirection::LeftToRight,
      GlyphBaseline::Alphabetic,
    );

    assert_eq!(shaped.glyphs.len(), 1, "material icon ligature should shape to one glyph");
    assert_eq!(shaped.glyphs[0].cluster, 0);
  }
}
