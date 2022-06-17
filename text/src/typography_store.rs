use std::sync::{Arc, RwLock};

use algo::FrameCache;
use arcstr::Substr;
use lyon_path::geom::{euclid::num::Zero, Point, Rect, Size};

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
  pub letter_space: Option<Pixel>,
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
  bounds: Size<Em>,
  visual_info: Arc<VisualInfos>,
}

struct ShapeRun {
  shape_result: Arc<ShapeResult>,
  font_size: FontSize,
  letter_space: Option<Pixel>,
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
    let mut bounds = cfg.bounds / em_font_size;

    if let Some(res) = self.get_from_cache(text.clone(), font_size, &cfg) {
      if !res.infos.over_bounds || res.bounds == bounds || res.bounds.greater_than(bounds).all() {
        return VisualGlyphs {
          scale: font_size.into_em().value(),
          bounds,
          visual_info: res.infos,
        };
      }

      // use the larger bounds as cache.
      bounds.width = bounds.width.max(res.bounds.width);
      bounds.height = bounds.height.max(res.bounds.height);
    }

    let input = Self::key(text, font_size, &cfg);

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
          font_size: FontSize::Em(Em::absolute(1.0)),
          letter_space: input.letter_space,
        }
      });

      InputParagraph { text_align: input.text_align, runs }
    });
    let t_cfg = TypographyCfg {
      line_height: input.line_height,
      letter_space: input.letter_space,
      text_align: input.text_align,
      bounds,
      line_dir: input.line_dir,
      overflow: input.overflow,
    };
    let t_man = TypographyMan::new(inputs, t_cfg);
    let visual_info = t_man.typography_all();
    let visual_info = Arc::new(visual_info);
    self.cache.write().unwrap().insert(
      input,
      TypographyResult { bounds, infos: visual_info.clone() },
    );
    VisualGlyphs {
      scale: font_size.into_em().value(),
      bounds,
      visual_info,
    }
  }

  fn get_from_cache(
    &self,
    text: Substr,
    font_size: FontSize,
    cfg: &TypographyCfg,
  ) -> Option<TypographyResult> {
    let input = Self::key(text, font_size, cfg);
    self.cache.read().unwrap().get(&input).cloned()
  }

  fn key(text: Substr, font_size: FontSize, cfg: &TypographyCfg) -> TypographyKey {
    let &TypographyCfg {
      line_height,
      text_align,
      line_dir,
      overflow,
      letter_space,
      ..
    } = cfg;
    let line_height = line_height.map(|l| l / font_size.into_em());
    let letter_space = letter_space.map(|l| l / font_size.into_pixel());

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

    TypographyKey {
      line_height,
      line_width,
      letter_space,
      text_align,
      line_dir,
      overflow,
      text,
    }
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
  fn letter_space(&self) -> Option<Pixel> { self.letter_space }

  #[inline]
  fn line_height(&self) -> Em { self.shape_result.line_height }
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
    self
      .visual_info
      .visual_lines
      .iter()
      .flat_map(move |l| l.glyphs.iter())
      .filter(|g| {
        Em::zero() <= g.x_offset + g.x_advance
          && g.x_offset < self.bounds.width
          && Em::zero() <= g.y_offset + g.y_advance
          && g.y_offset < self.bounds.height
      })
      .map(move |g| self.scale_to_pixel_glyph(g))
  }

  fn scale_to_pixel_glyph(&self, g: &Glyph<Em>) -> Glyph<Pixel> {
    let scale = self.scale;
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
  }
}

#[cfg(test)]
mod tests {
  use arcstr::{literal_substr, Substr};

  use super::*;
  use crate::{shaper::*, FontFace, FontFamily};

  fn test_face() -> FontFace {
    FontFace {
      families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
      ..<_>::default()
    }
  }
  fn test_store() -> TypographyStore {
    let font_db = Arc::new(RwLock::new(FontDB::default()));
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = font_db.write().unwrap().load_font_file(path);
    let shaper = TextShaper::new(font_db.clone());
    TypographyStore::new(<_>::default(), font_db, shaper)
  }

  fn typography_text(text: Substr, font_size: FontSize, cfg: TypographyCfg) -> VisualGlyphs {
    let store = test_store();

    store.typography(text, font_size, &test_face(), cfg)
  }

  #[test]
  fn simple_text_bounds() {
    let text = literal_substr!(
      "Hello
    
    
    
    world!"
    );

    let visual = typography_text(
      text,
      FontSize::Pixel(14.0.into()),
      TypographyCfg {
        letter_space: None,
        text_align: None,
        line_height: None,
        bounds: (Em::MAX, Em::MAX).into(),
        line_dir: PlaceLineDirection::TopToBottom,
        overflow: Overflow::Clip,
      },
    );

    assert_eq!(visual.visual_rect().size, Size::new(61.960938, 81.484375));
  }

  #[test]
  fn simple_typography_text() {
    fn glyphs(cfg: TypographyCfg) -> Vec<(f32, f32)> {
      let text = literal_substr!("Hello--------\nworld!");
      let info = typography_text(text, FontSize::Pixel(10.0.into()), cfg);
      info
        .pixel_glyphs()
        .map(|g| (g.x_offset.value(), g.y_offset.value()))
        .collect()
    }

    let mut cfg = TypographyCfg {
      letter_space: Some(Pixel::from(2.)),
      line_height: None,
      text_align: None,
      bounds: (Em::MAX, Em::MAX).into(),
      line_dir: PlaceLineDirection::TopToBottom,
      overflow: Overflow::Clip,
    };

    let not_bounds = glyphs(cfg.clone());
    assert_eq!(
      &not_bounds,
      &[
        (0.0, 0.0),
        (7.6445313, 0.0),
        (13.921876, 0.0),
        (16.825197, 0.0),
        (19.728518, 0.0),
        (26.157228, 0.0),
        (29.890629, 0.0),
        (33.624027, 0.0),
        (37.357426, 0.0),
        (41.09082, 0.0),
        (44.82422, 0.0),
        (48.557617, 0.0),
        (52.29101, 0.0),
        (56.024406, 0.0),
        // second line
        (0.0, 11.640625),
        (8.303711, 11.640625),
        (14.546876, 11.640625),
        (18.783205, 11.640625),
        (21.686525, 11.640625),
        (28.159182, 11.640625)
      ]
    );

    cfg.text_align = Some(TextAlign::End);
    cfg.bounds.width = Pixel(100.0.into()).into();
    let r_align = glyphs(cfg.clone());
    assert_eq!(
      &r_align,
      &[
        (37.974617, 0.0),
        (45.619144, 0.0),
        (51.896492, 0.0),
        (54.799812, 0.0),
        (57.703133, 0.0),
        (64.13184, 0.0),
        (67.86524, 0.0),
        (71.59865, 0.0),
        (75.33204, 0.0),
        (79.06544, 0.0),
        (82.798836, 0.0),
        (86.53223, 0.0),
        (90.265625, 0.0),
        (93.99902, 0.0),
        // second line
        (67.83203, 11.640625),
        (76.13574, 11.640625),
        (82.37891, 11.640625),
        (86.615234, 11.640625),
        (89.518555, 11.640625),
        (95.99121, 11.640625)
      ],
    );

    cfg.text_align = None;
    cfg.line_dir = PlaceLineDirection::BottomToTop;
    cfg.bounds.height = Pixel(100.0.into()).into();
    let bottom = glyphs(cfg.clone());

    assert_eq!(
      &bottom,
      &[
        // first line
        (0.0, 76.71875),
        (8.303711, 76.71875),
        (14.546876, 76.71875),
        (18.783205, 76.71875),
        (21.686525, 76.71875),
        (28.159182, 76.71875),
        // second line
        (0.0, 88.359375),
        (7.6445313, 88.359375),
        (13.921876, 88.359375),
        (16.825197, 88.359375),
        (19.728518, 88.359375),
        (26.157228, 88.359375),
        (29.890629, 88.359375),
        (33.624027, 88.359375),
        (37.357426, 88.359375),
        (41.09082, 88.359375),
        (44.82422, 88.359375),
        (48.557617, 88.359375),
        (52.29101, 88.359375),
        (56.024406, 88.359375)
      ],
    );

    cfg.text_align = Some(TextAlign::Center);
    cfg.line_dir = PlaceLineDirection::TopToBottom;
    cfg.bounds = Size::new(Pixel::from(40.).into(), Pixel::from(15.).into());
    let center_clip = glyphs(cfg);
    assert_eq!(
      &center_clip,
      &[
        (-3.3681612, 0.0),
        (2.9091835, 0.0),
        (5.8125043, 0.0),
        (8.715825, 0.0),
        (15.144537, 0.0),
        (18.877935, 0.0),
        (22.611334, 0.0),
        (26.344734, 0.0),
        (30.078129, 0.0),
        (33.811527, 0.0),
        (37.54492, 0.0),
        (3.9160144, 11.640625),
        (12.219725, 11.640625),
        (18.46289, 11.640625),
        (22.699219, 11.640625),
        (25.602541, 11.640625),
        (32.075195, 11.640625)
      ],
    );
  }

  #[test]
  fn cache_test() {
    let store = test_store();
    let cfg = TypographyCfg {
      line_height: None,
      letter_space: None,
      text_align: None,
      bounds: (Em::MAX, Em::MAX).into(),
      line_dir: PlaceLineDirection::TopToBottom,
      overflow: Overflow::Clip,
    };
    let text = literal_substr!("hi!");
    let font_size = FontSize::Em(Em::absolute(1.));
    assert!(
      store
        .get_from_cache(text.clone(), font_size, &cfg)
        .is_none()
    );

    let visual = store.typography(
      text.clone(),
      FontSize::Em(Em::absolute(1.0)),
      &test_face(),
      cfg.clone(),
    );

    assert_eq!(visual.pixel_glyphs().count(), 3);

    assert!(
      store
        .get_from_cache(text.clone(), font_size, &cfg)
        .is_some()
    );

    store.end_frame();
    store.end_frame();

    assert!(store.get_from_cache(text, font_size, &cfg).is_none());
  }
}
