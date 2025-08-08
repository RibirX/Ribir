use std::{
  cell::RefCell,
  hash::{Hash, Hasher},
};

use ribir_algo::{FrameCache, Sc, Substr};
pub use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{GlyphInfo, UnicodeBuffer};

use super::{GlyphUnit, font_db::GlyphBaseline};
use crate::{
  Glyph, TextDirection,
  font_db::{Face, FontDB, ID},
};

pub const NEWLINE_GLYPH_ID: GlyphId = GlyphId(u16::MAX);
/// Shaper to shape the `text` using provided font faces, and will do BIDI
/// reordering before to shape text.
///
/// This shaper will cache shaper result for per frame.
pub struct TextShaper {
  font_db: Sc<RefCell<FontDB>>,
  shape_cache: FrameCache<ShapeKey, Sc<ShapeResult>>,
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
  buffer: UnicodeBuffer,
}

impl TextShaper {
  #[inline]
  pub fn new(font_db: Sc<RefCell<FontDB>>) -> Self { Self { font_db, shape_cache: <_>::default() } }

  pub fn end_frame(&mut self) { self.shape_cache.end_frame("Text shape"); }

  /// Shape text and return the glyphs, caller should do text reorder before
  /// call this method.
  pub fn shape_text(
    &mut self, text: &Substr, face_ids: &[ID], direction: TextDirection, baseline: GlyphBaseline,
  ) -> Sc<ShapeResult> {
    if let Some(res) = self.get_cache(text, face_ids, direction, baseline) {
      res.clone()
    } else {
      let mut glyphs = self
        .shape_text_with_fallback(text, direction, face_ids, baseline)
        .unwrap_or_default();

      if let Some(last_char) = text.bytes().last() {
        if last_char == b'\r' || last_char == b'\n' {
          if let Some(g) = glyphs.last_mut() {
            g.glyph_id = NEWLINE_GLYPH_ID;
          }
        }
      }

      let glyphs = Sc::new(ShapeResult { text: text.clone(), glyphs });
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
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(dir.into());

    let GlyphsWithoutFallback { mut glyphs, mut buffer } =
      Self::directly_shape(buffer, baseline, dir.is_horizontal(), &face);
    let mut new_part = vec![(0, glyphs.len(), font_fallback.clone())];
    loop {
      if new_part.is_empty() {
        break;
      }
      let miss_part = collect_miss_part(&glyphs, &new_part);
      (buffer, new_part) = regen_miss_part(text, dir, baseline, &mut glyphs, miss_part, buffer);
    }

    Some(glyphs)
  }

  fn directly_shape(
    text: UnicodeBuffer, baseline: GlyphBaseline, hor_text: bool, face: &Face,
  ) -> GlyphsWithoutFallback {
    let output = rustybuzz::shape(face.as_rb_face(), &[], text);
    let mut glyphs = Vec::with_capacity(output.len());

    let infos = output.glyph_infos();
    let positions = output.glyph_positions();

    let shift = face.baseline_offset(baseline);
    let scale = GlyphUnit::UNITS_PER_EM as f32 / face.units_per_em() as f32;
    let shift = GlyphUnit::new(f32::ceil(shift as f32 * scale) as i32);

    (0..output.len()).for_each(|idx| {
      let &GlyphInfo { glyph_id, cluster, .. } = &infos[idx];
      let p = &positions[idx];
      let mut g = Glyph::new(GlyphId(glyph_id as u16), cluster, p, face);
      if hor_text {
        g.y_offset -= shift;
      } else {
        g.x_offset -= shift;
      }
      glyphs.push(g)
    });

    GlyphsWithoutFallback { glyphs, buffer: output.clear() }
  }

  pub fn get_cache(
    &mut self, text: &str, face_ids: &[ID], direction: TextDirection, baseline: GlyphBaseline,
  ) -> Option<Sc<ShapeResult>> {
    self
      .shape_cache
      .get(&(face_ids, text, direction, baseline) as &dyn ShapeKeySlice)
      .cloned()
  }

  pub fn font_db(&self) -> &Sc<RefCell<FontDB>> { &self.font_db }
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
  miss_part: Vec<(usize, usize, FallBackFaceHelper<'a>)>, mut buffer: UnicodeBuffer,
) -> (UnicodeBuffer, Vec<(usize, usize, FallBackFaceHelper<'a>)>) {
  let is_rtl = matches!(dir, TextDirection::RightToLeft | TextDirection::BottomToTop);
  let hb_direction = dir.into();

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
      buffer.push_str(miss_text);
      buffer.set_direction(hb_direction);
      let mut res = TextShaper::directly_shape(buffer, baseline, dir.is_horizontal(), &face);
      buffer = res.buffer;
      for g in res.glyphs.iter_mut() {
        g.cluster += miss_range.start as u32;
      }

      offset += (res.glyphs.len() as i32) - ((miss_end - miss_start) as i32);
      new_part.push((miss_start, miss_start + res.glyphs.len(), helper));
      glyphs.splice(miss_start..miss_end, res.glyphs);
    }
  }
  (buffer, new_part)
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

impl From<TextDirection> for rustybuzz::Direction {
  fn from(dir: TextDirection) -> Self {
    match dir {
      TextDirection::LeftToRight => rustybuzz::Direction::LeftToRight,
      TextDirection::RightToLeft => rustybuzz::Direction::RightToLeft,
      TextDirection::TopToBottom => rustybuzz::Direction::TopToBottom,
      TextDirection::BottomToTop => rustybuzz::Direction::BottomToTop,
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
    let font_db = self.font_db.borrow();
    loop {
      if self.face_idx >= self.ids.len() {
        return None;
      }

      let face = self
        .ids
        .get(self.face_idx)
        .and_then(|id| font_db.try_get_face_data(*id))
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
  use crate::{FontFace, FontFamily};

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
    let font_db = Sc::new(RefCell::new(FontDB::default()));
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
    assert!(res.glyphs.len() == 7);
  }
}
