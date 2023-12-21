use std::{
  cell::RefCell,
  hash::{Hash, Hasher},
  rc::Rc,
};

use crate::{
  font_db::{Face, FontDB, ID},
  Em, Glyph, TextDirection,
};
use ribir_algo::{FrameCache, Substr};

pub use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{GlyphInfo, UnicodeBuffer};

pub const NEWLINE_GLYPH_ID: GlyphId = GlyphId(core::u16::MAX);
/// Shaper to shape the `text` using provided font faces, and will do BIDI
/// reordering before to shape text.
///
/// This shaper will cache shaper result for per frame.
#[derive(Clone)]
pub struct TextShaper {
  font_db: Rc<RefCell<FontDB>>,
  shape_cache: Rc<RefCell<FrameCache<ShapeKey, Rc<ShapeResult>>>>,
}

#[derive(Debug, Clone)]
pub struct ShapeResult {
  pub text: Substr,
  pub glyphs: Vec<Glyph<Em>>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct ShapeKey {
  face_ids: Box<[ID]>,
  text: Substr,
  direction: TextDirection,
}

struct GlyphsWithoutFallback {
  glyphs: Vec<Glyph<Em>>,
  buffer: UnicodeBuffer,
}

impl TextShaper {
  #[inline]
  pub fn new(font_db: Rc<RefCell<FontDB>>) -> Self { Self { font_db, shape_cache: <_>::default() } }

  pub fn end_frame(&mut self) { self.shape_cache.borrow_mut().end_frame("Text shape"); }

  /// Shape text and return the glyphs, caller should do text reorder before
  /// call this method.
  pub fn shape_text(
    &self,
    text: &Substr,
    face_ids: &[ID],
    direction: TextDirection,
  ) -> Rc<ShapeResult> {
    self
      .get_from_cache(text, face_ids, direction)
      .unwrap_or_else(|| {
        let mut glyphs = self
          .shape_text_with_fallback(text, direction, face_ids)
          .unwrap_or_default();

        if let Some(last_char) = text.bytes().last() {
          if last_char == b'\r' || last_char == b'\n' {
            if let Some(g) = glyphs.last_mut() {
              g.glyph_id = NEWLINE_GLYPH_ID;
            }
          }
        }

        let glyphs = Rc::new(ShapeResult { text: text.clone(), glyphs });
        self.shape_cache.borrow_mut().put(
          ShapeKey {
            face_ids: face_ids.into(),
            text: text.clone(),
            direction,
          },
          glyphs.clone(),
        );
        glyphs
      })
  }

  /// Directly shape text without bidi reordering.
  pub fn shape_text_with_fallback(
    &self,
    text: &str,
    dir: TextDirection,
    face_ids: &[ID],
  ) -> Option<Vec<Glyph<Em>>> {
    let mut font_fallback = FallBackFaceHelper::new(face_ids, &self.font_db);
    let face = font_fallback.next_fallback_face(text)?;
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(dir.into());

    let GlyphsWithoutFallback { mut glyphs, mut buffer } = Self::directly_shape(buffer, &face);
    let mut new_part = vec![(0, glyphs.len(), font_fallback.clone())];
    loop {
      if new_part.is_empty() {
        break;
      }
      let miss_part = collect_miss_part(&glyphs, &new_part);
      (buffer, new_part) = regen_miss_part(text, dir, &mut glyphs, miss_part, buffer);
    }

    Some(glyphs)
  }

  fn directly_shape(text: UnicodeBuffer, face: &Face) -> GlyphsWithoutFallback {
    let output = rustybuzz::shape(face.as_rb_face(), &[], text);
    let mut glyphs = Vec::with_capacity(output.len());

    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    let units_per_em = face.units_per_em() as f32;
    (0..output.len()).for_each(|idx| {
      let &GlyphInfo { glyph_id, cluster, .. } = &infos[idx];
      let p = &positions[idx];
      glyphs.push(Glyph {
        face_id: face.face_id,
        x_advance: Em::absolute(p.x_advance as f32 / units_per_em),
        y_advance: Em::absolute(p.y_advance as f32 / units_per_em),
        x_offset: Em::absolute(p.x_offset as f32 / units_per_em),
        y_offset: Em::absolute(p.y_offset as f32 / units_per_em),
        glyph_id: GlyphId(glyph_id as u16),
        cluster,
      })
    });

    GlyphsWithoutFallback { glyphs, buffer: output.clear() }
  }

  pub fn get_from_cache(
    &self,
    text: &str,
    face_ids: &[ID],
    direction: TextDirection,
  ) -> Option<Rc<ShapeResult>> {
    self
      .shape_cache
      .borrow_mut()
      .get(&(face_ids, text, direction) as &(dyn ShapeKeySlice))
      .cloned()
  }

  pub fn font_db(&self) -> &Rc<RefCell<FontDB>> { &self.font_db }
}

fn collect_miss_part<'a>(
  glyphs: &[Glyph<Em>],
  new_part: &[(usize, usize, FallBackFaceHelper<'a>)],
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
          .map_or(true, |cluster| *cluster != glyph.cluster)
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
  text: &str,
  dir: TextDirection,
  glyphs: &mut Vec<Glyph<Em>>,
  miss_part: Vec<(usize, usize, FallBackFaceHelper<'a>)>,
  mut buffer: UnicodeBuffer,
) -> (UnicodeBuffer, Vec<(usize, usize, FallBackFaceHelper<'a>)>) {
  let is_rtl = matches!(dir, TextDirection::RightToLeft | TextDirection::BottomToTop);
  let hb_direction = dir.into();

  let cluster_to_range_byte = |glyphs: &Vec<Glyph<Em>>, idx: usize| -> usize {
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
      let mut res = TextShaper::directly_shape(buffer, &face);
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
}

impl<'a> std::borrow::Borrow<dyn ShapeKeySlice + 'a> for ShapeKey {
  fn borrow(&self) -> &(dyn ShapeKeySlice + 'a) { self }
}

impl Hash for dyn ShapeKeySlice + '_ {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.face_ids().hash(state);
    self.text().hash(state);
    self.direction().hash(state);
  }
}

impl PartialEq for dyn ShapeKeySlice + '_ {
  fn eq(&self, other: &Self) -> bool {
    self.face_ids() == other.face_ids()
      && self.text() == other.text()
      && self.direction() == other.direction()
  }
}

impl Eq for dyn ShapeKeySlice + '_ {}

impl ShapeKeySlice for ShapeKey {
  fn face_ids(&self) -> &[ID] { &self.face_ids }

  fn text(&self) -> &str { &self.text }

  fn direction(&self) -> TextDirection { self.direction }
}

impl ShapeKeySlice for (&[ID], &str, TextDirection) {
  fn face_ids(&self) -> &[ID] { self.0 }

  fn text(&self) -> &str { self.1 }

  fn direction(&self) -> TextDirection { self.2 }
}

fn is_miss_glyph_id(id: u16) -> bool { id == 0 }

impl<U: std::ops::MulAssign<f32>> Glyph<U> {
  fn is_miss(&self) -> bool { is_miss_glyph_id(self.glyph_id.0) }

  #[allow(unused)]
  fn is_not_miss(&self) -> bool { !self.is_miss() }

  pub fn scale(&mut self, scale: f32) {
    self.x_advance *= scale;
    self.y_advance *= scale;
    self.x_offset *= scale;
    self.y_offset *= scale;
  }
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
  ids: &'a [ID],
  font_db: &'a RefCell<FontDB>,
  face_idx: usize,
}

impl<'a> FallBackFaceHelper<'a> {
  fn new(ids: &'a [ID], font_db: &'a RefCell<FontDB>) -> Self { Self { ids, font_db, face_idx: 0 } }

  fn next_fallback_face(&mut self, text: &str) -> Option<Face> {
    let font_db = self.font_db.borrow();
    loop {
      if self.face_idx > self.ids.len() {
        return None;
      }
      let face_idx = self.face_idx;
      self.face_idx += 1;
      if face_idx == self.ids.len() {
        return font_db.try_get_face_data(font_db.default_font()).cloned();
      }

      let face = self
        .ids
        .get(face_idx)
        .and_then(|id| font_db.try_get_face_data(*id))
        .filter(|f| match text.is_empty() {
          true => true,
          false => text.chars().any(|c| f.has_char(c)),
        })
        .cloned();
      if face.is_some() {
        return face;
      }
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{FontFace, FontFamily};
  extern crate test;
  use test::Bencher;

  #[test]
  fn smoke() {
    let mut shaper = TextShaper::new(<_>::default());
    shaper.font_db.borrow_mut().load_system_fonts();

    let text: Substr = concat!["א", "ב", "ג", "a", "b", "c",].into();
    let ids = shaper.font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });
    let dir = TextDirection::LeftToRight;

    // No cache exists
    assert!(shaper.get_from_cache(&text, &ids, dir).is_none());

    let result = shaper.shape_text(&text, &ids, dir);
    assert_eq!(result.glyphs.len(), 6);

    assert!(shaper.get_from_cache(&text, &ids, dir).is_some());

    shaper.end_frame();
    shaper.end_frame();
    assert!(shaper.get_from_cache(&text, &ids, dir).is_none());
  }

  #[test]
  fn font_fallback() {
    let shaper = TextShaper::new(<_>::default());
    let path = env!("CARGO_MANIFEST_DIR").to_owned();
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path.clone() + "/../fonts/DejaVuSans.ttf");
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path + "/../fonts/NotoSerifSC-Bold.你好世界.otf");

    let ids_latin = shaper.font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
      ..<_>::default()
    });

    let ids_all = shaper.font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([
        FontFamily::Name("DejaVu Sans".into()),
        FontFamily::Name("Noto Serif SC".into()),
      ]),
      ..<_>::default()
    });

    let dir = TextDirection::LeftToRight;
    let latin1 = shaper.shape_text(&"hello world! 你好，世界".into(), &ids_latin, dir);
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

    let fallback_chinese = shaper.shape_text(&"hello world! 你好，世界".into(), &ids_all, dir);
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
    assert_eq!(
      &clusters,
      &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 19, 22, 25]
    );
  }

  #[test]
  fn shape_miss_font() {
    let shaper = TextShaper::new(<_>::default());

    let dir = TextDirection::LeftToRight;
    let result = shaper.shape_text(&"你好世界".into(), &[], dir);
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
    let shaper = TextShaper::new(font_db.clone());

    let text: Substr = "р҈р҈р҈р҈".into();

    {
      let ids = shaper.font_db.borrow_mut().select_all_match(&FontFace {
        families: Box::new([
          FontFamily::Name("GaramondNo8".into()),
          FontFamily::Name("Nunito".into()),
        ]),
        ..<_>::default()
      });
      let res = shaper.shape_text(&text.substr(..), &ids, TextDirection::LeftToRight);
      assert_eq!(res.glyphs.len(), 8);
      assert!(res.glyphs.iter().all(|glyph| glyph.is_miss()));
    }

    {
      let _ = font_db
        .borrow_mut()
        .load_font_file(env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf");
      let ids = shaper.font_db.borrow_mut().select_all_match(&FontFace {
        families: Box::new([
          FontFamily::Name("GaramondNo8".into()),
          FontFamily::Name("Nunito".into()),
          FontFamily::Name("DejaVu Sans".into()),
        ]),
        ..<_>::default()
      });
      shaper.shape_cache.borrow_mut().clear();
      let res = shaper.shape_text(&text.substr(..), &ids, TextDirection::LeftToRight);
      assert!(res.glyphs.len() == 8);
      assert!(res.glyphs.iter().all(|glyph| glyph.is_not_miss()));
    }
  }

  #[bench]
  fn shape_1k(bencher: &mut Bencher) {
    let shaper = TextShaper::new(<_>::default());
    shaper.font_db.borrow_mut().load_system_fonts();

    let ids = shaper.font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

    bencher.iter(|| {
      shaper.shape_cache.borrow_mut().clear();
      let str = include_str!("../../LICENSE").into();
      shaper.shape_text(&str, &ids, TextDirection::LeftToRight)
    })
  }

  #[test]
  fn shape_compose_emoji() {
    let shaper = TextShaper::new(<_>::default());
    let path = env!("CARGO_MANIFEST_DIR").to_owned();
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path.clone() + "/../fonts/DejaVuSans.ttf");
    let _ = shaper
      .font_db
      .borrow_mut()
      .load_font_file(path + "/../fonts/NotoSerifSC-Bold.你好世界.otf");
    let ids_all = shaper.font_db.borrow_mut().select_all_match(&FontFace {
      families: Box::new([
        FontFamily::Name("DejaVu Sans".into()),
        FontFamily::Name("Noto Serif SC".into()),
      ]),
      ..<_>::default()
    });

    let res = shaper.shape_text(&"👨‍👩‍👦‍👦".into(), &ids_all, TextDirection::LeftToRight);
    assert!(res.glyphs.len() == 7);
  }
}
