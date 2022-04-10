use std::sync::{Arc, RwLock};

use algo::FrameCache;
use arcstr::Substr;
use lyon_path::geom::{Point, Rect, Size};

use crate::{
  font_db::FontDB,
  shaper::{ShapeResult, TextShaper},
  typography::{
    InputParagraph, InputRun, Overflow, PlaceLineDirection, TypographyCfg, TypographyMan,
    VisualInfos,
  },
  Em, FontFace, FontSize, Glyph, Pixel, TextAlign, TextDirection, TextReorder,
};

/// Typography `text` relative to 1em.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypographyKey {
  pub line_height: Option<Em>,
  /// The max width of a line can be used to place glyph, we can use this to
  /// detect if a cache can be reuse even if its bounds is different.
  pub line_width: Em,
  pub letter_space: Option<Em>,
  pub text_align: Option<TextAlign>,
  pub line_dir: PlaceLineDirection,
  pub overflow: Overflow,
  pub text: Substr,
}

#[derive(Clone)]
struct TypographyResult {
  // The rect glyphs can place, and hint `TypographyMan` where to early return. The result of
  // typography may over bounds.
  pub bounds: Size<Em>,
  pub infos: Arc<VisualInfos>,
}

/// Do simple text typography and cache it.
#[derive(Clone)]
pub struct TypographyStore {
  reorder: TextReorder,
  shaper: TextShaper,
  font_db: Arc<RwLock<FontDB>>,
  cache: Arc<RwLock<FrameCache<TypographyKey, TypographyResult>>>,
}

pub struct VisualGlyphs {
  scale: f32,
  visual_info: Arc<VisualInfos>,
}

struct ShapeRun {
  shape_result: Arc<ShapeResult>,
  font_size: FontSize,
  letter_space: Option<Em>,
}

impl TypographyStore {
  pub fn new(reorder: TextReorder, font_db: Arc<RwLock<FontDB>>, shaper: TextShaper) -> Self {
    TypographyStore {
      reorder,
      shaper,
      font_db,
      cache: <_>::default(),
    }
  }

  pub fn end_frame(&self) { self.cache.write().unwrap().end_frame("Typography"); }

  pub fn typography(
    &self,
    text: Substr,
    font_size: FontSize,
    face: &FontFace,
    cfg: TypographyCfg,
  ) -> VisualGlyphs {
    let em_font_size = font_size.into_em();
    let TypographyCfg {
      line_height,
      text_align,
      bounds,
      line_dir,
      overflow,
      letter_space,
    } = cfg;
    let line_height = line_height.map(|l| l / em_font_size);
    let letter_space = letter_space.map(|l| l / em_font_size);
    let mut bounds = bounds / em_font_size;
    let line_width = match overflow {
      // line width is not so important in clip mode, the cache can be use even with difference line
      // width. The wider one can use for the narrower one. S
      Overflow::Clip => Em::absolute(f32::MAX),
      // For word wrap
      //
      // if line_dir.is_horizontal() {
      //   bounds.width()
      // } else {
      //   bounds.height()
      // };
    };

    let input = TypographyKey {
      line_height,
      line_width,
      letter_space,
      text_align,
      line_dir,
      overflow,
      text,
    };
    if let Some(res) = self.get_from_cache(&input) {
      if !res.infos.over_bounds || res.bounds == bounds || res.bounds.greater_than(bounds).all() {
        let visual_info = res.infos.clone();
        return VisualGlyphs {
          scale: font_size.into_em().value(),
          visual_info,
        };
      }

      // use the larger bounds as cache.
      bounds.width = bounds.width.max(res.bounds.width);
      bounds.height = bounds.height.max(res.bounds.height);
    }

    let info = self.reorder.reorder_text(&input.text);
    let ids = self.font_db.read().unwrap().select_all_match(face);
    let inputs = info.paras.iter().map(|p| {
      let runs = p.runs.iter().map(|r| {
        let dir = if p.levels[r.start].is_ltr() {
          TextDirection::LeftToRight
        } else {
          TextDirection::RightToLeft
        };

        let shape_result = self
          .shaper
          .shape_text(&input.text.substr(r.clone()), &ids, dir);

        ShapeRun {
          shape_result,
          font_size,
          letter_space,
        }
      });

      InputParagraph { text_align, runs }
    });
    let t_cfg = TypographyCfg {
      line_height,
      letter_space,
      text_align,
      bounds,
      line_dir,
      overflow,
    };
    let t_man = TypographyMan::new(inputs, t_cfg, self.font_db.clone());
    let visual_info = t_man.typography_all();
    let visual_info = Arc::new(visual_info);
    self.cache.write().unwrap().insert(
      input,
      TypographyResult { bounds, infos: visual_info.clone() },
    );
    VisualGlyphs {
      scale: font_size.into_em().value(),
      visual_info,
    }
  }

  fn get_from_cache(&self, input: &TypographyKey) -> Option<TypographyResult> {
    self.cache.read().unwrap().get(input).cloned()
  }
}

impl InputRun for ShapeRun {
  #[inline]
  fn text(&self) -> &str { &self.shape_result.text }

  #[inline]
  fn glyphs(&self) -> &[Glyph<Em>] { &self.shape_result.glyphs }

  #[inline]
  fn font_size(&self) -> FontSize { self.font_size }

  #[inline]
  fn letter_space(&self) -> Option<Em> { self.letter_space }
}

impl VisualGlyphs {
  /// return a visual rect to place the text in pixel.
  pub fn visual_rect(&self) -> Rect<f32> {
    let em_rect = self.visual_info.box_rect;

    Rect::new(
      Point::new(
        self.to_pixel_value(em_rect.min_x()),
        self.to_pixel_value(em_rect.min_y()),
      ),
      Size::new(
        self.to_pixel_value(em_rect.width()),
        self.to_pixel_value(em_rect.height()),
      ),
    )
  }

  fn to_pixel_value(&self, v: Em) -> f32 {
    let p: Pixel = (v * self.scale).into();
    p.value()
  }

  pub fn pixel_glyphs(&self) -> impl Iterator<Item = Glyph<Pixel>> + '_ {
    self.visual_info.visual_lines.iter().flat_map(|l| {
      let scale = self.scale;
      l.glyphs.iter().map(move |g| {
        let Glyph {
          face_id,
          x_advance,
          y_advance,
          x_offset,
          y_offset,
          glyph_id,
          cluster,
        } = *g;
        Glyph {
          face_id,
          x_advance: (x_advance * scale).into(),
          y_advance: (y_advance * scale).into(),
          x_offset: (x_offset * scale).into(),
          y_offset: (y_offset * scale).into(),
          glyph_id,
          cluster,
        }
      })
    })
  }
}
