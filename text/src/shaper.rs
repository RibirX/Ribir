use std::{
  borrow::Borrow,
  hash::{Hash, Hasher},
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
  font_db::{Face, FontDB, ID},
  TextDirection,
};
use algo::FrameCache;

use arcstr::Substr;
use rustybuzz::{GlyphInfo, UnicodeBuffer};
pub use ttf_parser::GlyphId;

/// A glyph information returned by text shaped, include a glyph
#[derive(Debug, Clone)]
pub struct Glyph {
  /// The font face id of the glyph.
  pub face_id: ID,
  /// How many ems the line advances after drawing this glyph when setting text
  /// in horizontal direction.
  pub x_advance: f32,
  /// How many ems the line advances after drawing this glyph when setting text
  /// in vertical direction.
  pub y_advance: f32,
  /// How many ems the glyph moves on the X-axis before drawing it, this should
  /// not affect how many the line advances.
  pub x_offset: f32,
  /// How many ems the glyph moves on the Y-axis before drawing it, this should
  /// not affect how many the line advances.
  pub y_offset: f32,
  /// The id of the glyph.
  pub glyph_id: GlyphId,
  /// An cluster of origin text as byte index.
  pub cluster: u32,
}

/// Shaper to shape the `text` using provided font faces, and will do BIDI
/// reordering before to shape text.
///
/// This shaper will cache shaper result for per frame.
#[derive(Default, Clone)]
pub struct TextShaper {
  font_db: Arc<RwLock<FontDB>>,
  shape_cache: Arc<RwLock<FrameCache<ShapeKey, Arc<ShapeResult>>>>,
}

#[derive(Debug, Clone)]
pub struct ShapeResult {
  pub text: Substr,
  pub glyphs: Vec<Glyph>,
  pub direction: TextDirection,
  /// The biggest height of font use to shape the text in em. For vertical font
  /// is its "width".
  pub max_line_size: f32,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct ShapeKey {
  face_ids: Box<[ID]>,
  text: Substr,
  direction: TextDirection,
}

impl TextShaper {
  pub fn end_frame(&mut self) {
    self.shape_cache.write().unwrap().end_frame("Text shape");
    self.font_db.write().unwrap().end_frame()
  }

  /// Shape text and return the glyphs, caller should do text reorder before
  /// call this method.
  pub fn shape_text(
    &self,
    text: &Substr,
    face_ids: &[ID],
    direction: TextDirection,
  ) -> Arc<ShapeResult> {
    self
      .get_from_cache(text, face_ids, direction)
      .unwrap_or_else(|| {
        let (glyphs, line_height) = if !cover_all_glyphs(text, face_ids, &*self.font_db) {
          log::warn!(
            "Text shape: some glyphs not covered in the text: {}",
            &**text
          );
          let ids = db_fallback_fonts(face_ids, &*self.font_db);

          self.shape_text_with_fallback(text, direction, &ids, &mut None)
        } else {
          self.shape_text_with_fallback(text, direction, face_ids, &mut None)
        }
        .unwrap_or_else(|| {
          log::warn!("There is no font can shape the text: \"{}\"", &**text);
          // if no font can shape the text use the first font shape it with miss glyph.
          let face = {
            let mut font_db = self.font_db_mut();
            face_ids
              .iter()
              .find_map(|id| font_db.face_data_or_insert(*id).cloned())
              .unwrap_or_else(|| {
                font_db
                  .faces()
                  .iter()
                  .find_map(|info| font_db.try_get_face_data(info.id))
                  .expect("No font can use.")
                  .clone()
              })
          };
          let (glyphs, _) = Self::directly_shape(text, direction, &face, &mut None);

          (glyphs, line_size(direction, &face))
        });

        let glyphs = Arc::new(ShapeResult {
          text: text.clone(),
          glyphs,
          max_line_size: line_height,
          direction,
        });
        self.shape_cache.write().unwrap().insert(
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
    // todo: remove it
    buffer: &mut Option<UnicodeBuffer>,
  ) -> Option<(Vec<Glyph>, f32)> {
    let (id_idx, face) = { self.font_db_mut().shapeable_face(text, face_ids) }?;

    let (mut glyphs, mut miss_from) = Self::directly_shape(text, dir, &face, buffer);
    let mut line_height = line_size(dir, &face);
    // todo: we need align baseline.
    while let Some(m_start) = miss_from {
      let m_end = glyphs[m_start..]
        .iter()
        .position(Glyph::is_not_miss)
        .map(|i| m_start + i);

      let start_byte = glyphs[m_start].cluster as usize;
      let miss_text = match m_end {
        Some(miss_end) => {
          let end_byte = glyphs[miss_end].cluster as usize;
          &text[start_byte..end_byte]
        }
        None => &text[start_byte..],
      };

      let fallback_glyphs =
        self.shape_text_with_fallback(miss_text, dir, &face_ids[id_idx + 1..], buffer);

      if let Some((fallback, lh)) = fallback_glyphs {
        match m_end {
          Some(m_end) => glyphs.splice(m_start..m_end, fallback),
          None => glyphs.splice(m_start.., fallback),
        };
        line_height = line_height.max(lh);
      }

      // skip to next miss glyphs
      miss_from = m_end.and_then(|idx| {
        glyphs[idx..]
          .iter()
          .position(Glyph::is_miss)
          .map(|i| i + idx)
      });
    }

    Some((glyphs, line_height))
  }

  pub fn directly_shape(
    text: &str,
    direction: TextDirection,
    face: &Face,
    buffer: &mut Option<UnicodeBuffer>,
  ) -> (Vec<Glyph>, Option<usize>) {
    let mut run_buffer = buffer.take().unwrap_or_default();
    run_buffer.push_str(text);
    let hb_direction = match direction {
      TextDirection::LeftToRight => rustybuzz::Direction::LeftToRight,
      TextDirection::RightToLeft => rustybuzz::Direction::RightToLeft,
      TextDirection::TopToBottom => rustybuzz::Direction::TopToBottom,
      TextDirection::BottomToTop => rustybuzz::Direction::BottomToTop,
    };
    run_buffer.set_direction(hb_direction);
    let output = rustybuzz::shape(face.as_rb_face(), &[], run_buffer);

    let mut miss_from = None;

    let mut glyphs = Vec::with_capacity(output.len());

    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    let units_per_em = face.units_per_em() as f32;
    (0..output.len()).for_each(|idx| {
      let &GlyphInfo { glyph_id, cluster, .. } = &infos[idx];
      let p = &positions[idx];
      if miss_from.is_none() && is_miss_glyph_id(glyph_id as u16) {
        miss_from = Some(idx);
      }
      glyphs.push(Glyph {
        face_id: face.face_id,
        x_advance: p.x_advance as f32 / units_per_em,
        y_advance: p.y_advance as f32 / units_per_em,
        x_offset: p.x_offset as f32 / units_per_em,
        y_offset: p.y_offset as f32 / units_per_em,
        glyph_id: GlyphId(glyph_id as u16),
        cluster,
      })
    });

    buffer.replace(output.clear());

    (glyphs, miss_from)
  }

  pub fn get_from_cache(
    &self,
    text: &str,
    face_ids: &[ID],
    direction: TextDirection,
  ) -> Option<Arc<ShapeResult>> {
    self
      .shape_cache
      .read()
      .unwrap()
      .get(&(face_ids, text, direction) as &(dyn ShapeKeySlice))
      .cloned()
  }

  pub fn font_db(&self) -> RwLockReadGuard<'_, FontDB> { self.font_db.read().unwrap() }

  pub fn font_db_mut(&self) -> RwLockWriteGuard<FontDB> { self.font_db.write().unwrap() }
}

fn cover_all_glyphs(text: &str, ids: &[ID], font_db: &RwLock<FontDB>) -> bool {
  let mut faces = Vec::with_capacity(ids.len());
  let mut lazy_faces = ids.iter().filter_map(|id| {
    let mut db = font_db.write().unwrap();
    db.face_data_or_insert(*id).cloned()
  });

  if let Some(f) = lazy_faces.next() {
    faces.push(f);
  }

  text.chars().all(move |c| {
    faces.iter_mut().any(|f| f.has_char(c))
      || lazy_faces.any(|f| {
        faces.push(f);
        faces.last().unwrap().has_char(c)
      })
  })
}

fn line_size(dir: TextDirection, face: &Face) -> f32 {
  let height = match dir {
    TextDirection::LeftToRight | TextDirection::RightToLeft => face.height(),
    TextDirection::TopToBottom | TextDirection::BottomToTop => {
      face.vertical_height().unwrap_or_else(|| face.height())
    }
  };
  height as f32 / face.units_per_em() as f32
}

fn db_fallback_fonts(high_prior: &[ID], font_db: &RwLock<FontDB>) -> Vec<ID> {
  let db = font_db.read().unwrap();
  let faces = db.faces();
  let mut ids = Vec::with_capacity(faces.len());
  ids.extend(high_prior);
  for f in faces {
    if high_prior.iter().all(|id| *id != f.id) {
      ids.push(f.id);
    }
  }
  ids
}
trait ShapeKeySlice {
  fn face_ids(&self) -> &[ID];
  fn text(&self) -> &str;
  fn direction(&self) -> TextDirection;
}

impl<'a> Borrow<dyn ShapeKeySlice + 'a> for ShapeKey {
  fn borrow(&self) -> &(dyn ShapeKeySlice + 'a) { self }
}

impl Hash for dyn ShapeKeySlice + '_ {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.face_ids().hash(state);
    self.text().hash(state);
  }
}

impl PartialEq for dyn ShapeKeySlice + '_ {
  fn eq(&self, other: &Self) -> bool {
    self.face_ids() == other.face_ids() && self.text() == other.text()
  }
}

impl Eq for dyn ShapeKeySlice + '_ {}

impl ToOwned for dyn ShapeKeySlice + '_ {
  type Owned = ShapeKey;

  fn to_owned(&self) -> Self::Owned {
    ShapeKey {
      face_ids: self.face_ids().into(),
      text: self.text().to_owned().into(),
      direction: self.direction(),
    }
  }
}

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

impl Glyph {
  fn is_miss(&self) -> bool { is_miss_glyph_id(self.glyph_id.0) }

  fn is_not_miss(&self) -> bool { !self.is_miss() }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{FontFace, FontFamily};
  extern crate test;
  use test::Bencher;

  #[test]
  fn smoke() {
    let mut shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();

    let text = concat!["א", "ב", "ג", "a", "b", "c",];
    let ids = shaper.font_db().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

    // No cache exists
    assert!(shaper.get_from_cache(text, &ids).is_none());

    let lines = shaper.shape_text(text, &ids);
    assert_eq!(lines.len(), 1);

    let ParagraphShaped { levels, runs, .. } = &lines[0];
    assert_eq!(
      &levels.iter().map(|l| l.number()).collect::<Vec<_>>(),
      &[1, 1, 1, 1, 1, 1, 2, 2, 2]
    );

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].run, 6..9);
    assert_eq!(runs[0].glyphs.len(), 3);

    assert_eq!(runs[1].run, 0..6);
    assert_eq!(runs[0].glyphs.len(), 3);

    assert!(shaper.get_from_cache(text, &ids).is_some());

    shaper.end_frame();
    shaper.end_frame();
    assert!(shaper.get_from_cache(text, &ids).is_none());
  }

  #[test]
  fn font_fallback() {
    let shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = shaper.font_db_mut().load_font_file(path);

    let ids = shaper.font_db().select_all_match(&FontFace {
      families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
      ..<_>::default()
    });

    let shape_latin = shaper.shape_text("hello world!", &ids);
    let latin1 = shape_latin[0].runs[0].glyphs.as_ref();
    assert_eq!(latin1.len(), 12);
    let fallback_chinese = shaper.shape_text("hello world! 你好，世界", &ids);
    let latin2 = fallback_chinese[0].runs[0].glyphs.as_ref();
    let b = &latin2[..latin1.len()];
    assert!(Iterator::eq(
      latin1.iter().map(|g| { (g.glyph_id, g.x_advance) }),
      b.iter().map(|g| { (g.glyph_id, g.x_advance) })
    ));
  }

  #[test]
  fn slice_unicode() {
    let shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();

    let text = "⚛∩∗∔⋅⋖⊵⊶⊇≺∹⊈⋫⋷⋝⊿⋌⊷⋖⊐≑⊢⊷⋧";
    let ids = shaper.font_db().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

    shaper.shape_cache.write().unwrap().clear();
    let glyphs = shaper.shape_text(text, &ids);
    assert_eq!(glyphs[0].runs[0].glyphs.as_ref().len(), 24);
  }

  #[bench]
  fn shape_1k(bencher: &mut Bencher) {
    let shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();

    let text = include_str!("../../LICENSE");
    let ids = shaper.font_db().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

    bencher.iter(|| {
      shaper.shape_cache.write().unwrap().clear();
      shaper.shape_text(text, &ids)
    })
  }
}
