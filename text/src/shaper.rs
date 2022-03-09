use std::{
  borrow::Borrow,
  hash::{Hash, Hasher},
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::font_db::{self, Face, FontDB, ID};
use algo::{CowRc, FrameCache};

use rustybuzz::{GlyphInfo, GlyphPosition, UnicodeBuffer};
pub use ttf_parser::GlyphId;
use unic_bidi::{BidiInfo, Level, LevelRun};

/// A glyph information returned by text shaped, include a glyph
#[derive(Debug, Clone)]
pub struct Glyph {
  /// The font face id of the glyph.
  pub face_id: ID,
  /// How much em the line advances after drawing this glyph when setting text
  /// in horizontal direction.
  pub x_advance: f32,
  /// How much em the line advances after drawing this glyph when setting text
  /// in vertical direction.
  pub y_advance: f32,
  /// How much ems the glyph moves on the X-axis before drawing it, this should
  /// not affect how much the line advances.
  pub x_offset: f32,
  /// How much em the glyph moves on the Y-axis before drawing it, this should
  /// not affect how much the line advances.
  pub y_offset: f32,
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
  shape_cache: Arc<RwLock<FrameCache<ShapeKey, Arc<[ParagraphShaped]>>>>,
}

#[derive(Debug, Clone)]
pub struct RunShaped {
  pub run: LevelRun,
  pub glyphs: Box<[Glyph]>,
  /// How much em the line need to drawing this glyph when setting text in
  /// horizontal direction.
  pub width: f32,
  /// How much em the line need to drawing this glyph when setting text in
  /// Vertical direction.
  pub heigh: f32,
}
/// Shape result of a paragraph, correspond to `ParagraphVisualRuns`
#[derive(Clone, Debug)]
pub struct ParagraphShaped {
  pub levels: Box<[Level]>,
  pub runs: Box<[RunShaped]>,
  /// How much em the line need to drawing this glyph when setting text in
  /// horizontal direction.
  pub width: f32,
  /// How much em the line need to drawing this glyph when setting text in
  /// Vertical direction.
  pub heigh: f32,
  /// The height of the first font use to shape the text in em.
  pub first_font_height: f32,
}

#[derive(PartialEq, Eq, Hash)]
struct ShapeKey {
  face_ids: Box<[ID]>,
  text: CowRc<str>,
}

impl TextShaper {
  pub fn end_frame(&mut self) {
    self.shape_cache.write().unwrap().end_frame("Text shape");
    self.font_db.write().unwrap().end_frame()
  }

  /// Shape text with bidi reordering.
  pub fn shape_text(&self, text: &str, face_ids: &[ID]) -> Arc<[ParagraphShaped]> {
    match self.get(text, face_ids) {
      Some(v) => v,
      None => {
        let glyphs: Arc<[ParagraphShaped]> = if !cover_all_glyphs(text, face_ids, &*self.font_db) {
          log::warn!("Text shape: some glyphs not covered in the text: {}", text);
          let ids = db_fallback_fonts(face_ids, &*self.font_db);
          self.reorder_and_shape(text, &ids).into()
        } else {
          self.reorder_and_shape(text, face_ids).into()
        };

        self.shape_cache.write().unwrap().insert(
          ShapeKey {
            face_ids: face_ids.into(),
            text: text.to_owned().into(),
          },
          glyphs.clone(),
        );
        glyphs
      }
    }
  }

  /// Directly shape text without bidi reordering.
  pub fn shape_text_without_bidi(
    &self,
    text: &str,
    is_rtl: bool,
    face_ids: &[ID],
    buffer: &mut Option<UnicodeBuffer>,
  ) -> Option<Vec<Glyph>> {
    let (id_idx, face) = { self.font_db_mut().shapeable_face(text, face_ids) }?;

    let (mut glyphs, mut miss_from) = Self::directly_shape(text, is_rtl, &face, buffer);
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
        self.shape_text_without_bidi(miss_text, is_rtl, &face_ids[id_idx + 1..], buffer);

      if let Some(fallback) = fallback_glyphs {
        match m_end {
          Some(m_end) => glyphs.splice(m_start..m_end, fallback),
          None => glyphs.splice(m_start.., fallback),
        };
      }

      // skip to next miss glyphs
      miss_from = m_end.and_then(|idx| {
        glyphs[idx..]
          .iter()
          .position(Glyph::is_miss)
          .map(|i| i + idx)
      });
    }

    Some(glyphs)
  }

  pub fn directly_shape(
    text: &str,
    is_rtl: bool,
    face: &Face,
    buffer: &mut Option<UnicodeBuffer>,
  ) -> (Vec<Glyph>, Option<usize>) {
    let mut run_buffer = buffer.take().unwrap_or_default();
    run_buffer.push_str(text);
    let hb_direction = if is_rtl {
      rustybuzz::Direction::RightToLeft
    } else {
      rustybuzz::Direction::LeftToRight
    };
    run_buffer.set_direction(hb_direction);
    let output = rustybuzz::shape(face.as_rb_face(), &[], run_buffer);

    let mut miss_from = None;

    let mut glyphs = Vec::with_capacity(output.len());
    (0..output.len()).for_each(|g_idx| {
      let pos = output.glyph_positions()[g_idx];
      let info = &output.glyph_infos()[g_idx];
      let glyph = Glyph::new(face, pos, info);
      if miss_from.is_none() && glyph.is_miss() {
        miss_from = Some(g_idx);
      }
      glyphs.push(glyph);
    });
    buffer.replace(output.clear());

    (glyphs, miss_from)
  }

  pub fn get(&self, text: &str, face_ids: &[ID]) -> Option<Arc<[ParagraphShaped]>> {
    self
      .shape_cache
      .read()
      .unwrap()
      .get(&(face_ids, text) as &(dyn ShapeKeySlice))
      .cloned()
  }

  pub fn font_db(&self) -> RwLockReadGuard<'_, FontDB> { self.font_db.read().unwrap() }

  pub fn font_db_mut(&self) -> RwLockWriteGuard<FontDB> { self.font_db.write().unwrap() }

  fn reorder_and_shape(&self, text: &str, face_ids: &[ID]) -> Vec<ParagraphShaped> {
    let bidi_info = BidiInfo::new(text, None);
    let mut buffer = Some(UnicodeBuffer::new());
    let mut lines = Vec::with_capacity(bidi_info.paragraphs.len());

    bidi_info.paragraphs.iter().for_each(|p| {
      let line = p.range.clone();
      let (levels, runs) = bidi_info.visual_runs(p, line);
      let mut line = Vec::with_capacity(runs.len());

      let (mut line_w, mut line_h) = (0., 0.);
      for r in runs {
        let run_text = &text[r.clone()];
        let glyphs = self
          .shape_text_without_bidi(run_text, levels[r.start].is_rtl(), face_ids, &mut buffer)
          .unwrap_or_else(|| {
            // if not font can shape the text use the first font shape it with miss glyph.
            let face = {
              let mut font_db = self.font_db_mut();
              face_ids
                .iter()
                .find_map(|id| font_db.face_data_or_insert(*id).cloned())
                .expect("No font can use.")
            };
            let (glyphs, _) =
              Self::directly_shape(run_text, levels[r.start].is_rtl(), &face, &mut buffer);
            glyphs
          });
        let (width, heigh) = glyphs
          .iter()
          .map(|g| (g.x_offset + g.x_advance, g.y_offset + g.y_advance))
          .fold((0., 0.), |(sw, sh), (w, h)| (sw + w, sh + h));
        line_w += width;
        line_h += heigh;
        line.push(RunShaped {
          run: r.clone(),
          glyphs: glyphs.into_boxed_slice(),
          width,
          heigh,
        });
      }

      let g = line.iter().flat_map(|g| g.glyphs.iter()).next();
      let first_font_height = g.map_or(1., |g| {
        let db = self.font_db();
        let face = db.try_get_face_data(g.face_id).expect("font must existed.");
        face.height() as f32 / face.units_per_em() as f32
      });
      lines.push(ParagraphShaped {
        levels: levels.into_boxed_slice(),
        runs: line.into_boxed_slice(),
        width: line_w,
        heigh: line_h,
        first_font_height,
      });
    });

    lines
  }
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
    }
  }
}

impl ShapeKeySlice for ShapeKey {
  fn face_ids(&self) -> &[ID] { &self.face_ids }

  fn text(&self) -> &str { &self.text }
}

impl ShapeKeySlice for (&[ID], &str) {
  fn face_ids(&self) -> &[ID] { self.0 }

  fn text(&self) -> &str { self.1 }
}

impl Glyph {
  fn is_miss(&self) -> bool { self.glyph_id.0 == 0 }

  fn is_not_miss(&self) -> bool { !self.is_miss() }

  fn new(face: &font_db::Face, pos: GlyphPosition, info: &GlyphInfo) -> Self {
    let glyph_id = GlyphId(info.glyph_id as u16);
    let cluster = info.cluster;

    let units_per_em = face.units_per_em() as f32;
    Glyph {
      face_id: face.face_id,
      glyph_id,
      cluster,
      x_advance: pos.x_advance as f32 / units_per_em,
      y_advance: pos.y_advance as f32 / units_per_em,
      x_offset: pos.x_offset as f32 / units_per_em,
      y_offset: pos.y_offset as f32 / units_per_em,
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
    let mut shaper = TextShaper::default();
    shaper.font_db_mut().load_system_fonts();

    let text = concat!["א", "ב", "ג", "a", "b", "c",];
    let ids = shaper.font_db().select_all_match(&FontFace {
      families: Box::new([FontFamily::Serif, FontFamily::Cursive]),
      ..<_>::default()
    });

    // No cache exists
    assert!(shaper.get(text, &ids).is_none());

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

    assert!(shaper.get(text, &ids).is_some());

    shaper.end_frame();
    shaper.end_frame();
    assert!(shaper.get(text, &ids).is_none());
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
