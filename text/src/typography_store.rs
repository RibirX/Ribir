use std::{
  ops::Range,
  sync::{Arc, RwLock},
};

use lyon_path::geom::euclid::num::Zero;
use ribir_algo::{FrameCache, Substr};
use ribir_geom::{LogicUnit, Point, Rect, Size};

type SizeEm = lyon_path::geom::euclid::Size2D<Em, LogicUnit>;

use crate::{
  font_db::FontDB,
  shaper::{ShapeResult, TextShaper},
  text_reorder::ReorderResult,
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
  pub bounds: SizeEm,
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
  bounds: SizeEm,
  visual_info: Arc<VisualInfos>,
  order_info: Arc<ReorderResult>,
}

struct ShapeRun {
  shape_result: Arc<ShapeResult>,
  font_size: FontSize,
  letter_space: Option<Pixel>,
  range: Range<usize>,
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
          order_info: self.reorder.reorder_text(&text),
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
        let dir = if r.is_empty() || p.levels[r.start].is_ltr() {
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
          range: r.clone(),
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
      order_info: info,
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
  fn range(&self) -> Range<usize> { self.range.clone() }
}

impl VisualGlyphs {
  /// return a visual rect to place the text in pixel.
  pub fn visual_rect(&self) -> Rect {
    let info = &self.visual_info;

    Rect::new(
      Point::new(
        self.to_pixel_value(info.visual_x),
        self.to_pixel_value(info.visual_y),
      ),
      Size::new(
        self.to_pixel_value(info.visual_width),
        self.to_pixel_value(info.visual_height),
      ),
    )
  }

  pub fn nearest_glyph(&self, offset_x: f32, offset_y: f32) -> (usize, usize) {
    let rc = self.visual_rect();

    let mut bottom: Em = Pixel(rc.height().into()).into();

    let x: Em = Pixel(offset_x.into()).into();
    let y: Em = Pixel(offset_y.into()).into();

    let mut iter = self
      .visual_info
      .visual_lines
      .iter()
      .enumerate()
      .rev()
      .skip_while(move |(_, line)| {
        bottom = bottom.max(line.height) - line.height;
        y < bottom
      });

    if let Some((row, line)) = iter.next() {
      let idx = line
        .glyphs
        .iter()
        .enumerate()
        .rev()
        .find(|(_, g)| Em::zero() < g.x_advance && g.x_offset <= x)
        .map(|(i, g)| {
          if x - g.x_offset >= g.x_offset + g.x_advance - x {
            i + 1
          } else {
            i
          }
        })
        .unwrap_or(0);
      return (row, idx);
    }

    (0, 0)
  }

  pub fn position_by_cluster(&self, cluster: u32) -> (usize, usize) {
    struct RangeLocator<'a, Idx>
    where
      Idx: Ord + Copy,
    {
      ranges: Vec<(&'a Range<Idx>, usize)>,
    }

    impl<'a, Idx> RangeLocator<'a, Idx>
    where
      Idx: Ord + Copy,
    {
      fn from_unorder_ranges(rgs: impl Iterator<Item = &'a Range<Idx>>) -> Self
      where
        Idx: 'static,
      {
        let mut ranges: Vec<_> = rgs.enumerate().map(|(idx, item)| (item, idx)).collect();
        ranges.sort_by(|lh, rh| lh.0.start.cmp(&rh.0.start));
        RangeLocator { ranges }
      }

      fn range_index(&self, val: Idx) -> Option<usize> {
        let idx = self.ranges.partition_point(|item| item.0.end <= val);
        if idx < self.ranges.len() && self.ranges[idx].0.contains(&val) {
          Some(self.ranges[idx].1)
        } else {
          None
        }
      }
    }

    let visual_lines = &self.visual_info.visual_lines;
    let para = self
      .order_info
      .paras
      .partition_point(|p| p.range.end <= cluster as usize)
      .min(self.order_info.paras.len() - 1);
    let order_info = &self.order_info.paras[para];
    let line = &visual_lines[para];
    let locator = RangeLocator::from_unorder_ranges(order_info.runs.iter());
    if let Some(dst_run) = locator.range_index(cluster as usize) {
      let is_ltr = order_info.levels[order_info.runs[dst_run].start].is_ltr();
      let offset = line.glyphs.partition_point(|glyph| {
        let glyph_run = locator.range_index(glyph.cluster as usize);
        let glyph_run = glyph_run.unwrap();
        if dst_run == glyph_run {
          if is_ltr {
            return glyph.cluster < cluster;
          } else {
            return glyph.cluster > cluster;
          }
        }
        glyph_run < dst_run
      });
      (para, offset)
    } else {
      (para, order_info.range.end)
    }
  }

  pub fn position_to_cluster(&self, para: usize, offset: usize) -> u32 {
    let lines = &self.visual_info.visual_lines;
    let paras = &self.order_info.paras;
    if para >= lines.len() {
      return paras.last().map_or(0, |p| p.range.end as u32);
    } else {
      return lines[para]
        .glyphs
        .get(offset)
        .map_or(paras[para].range.end as u32, |g| g.cluster);
    }
  }

  pub fn glyph_rect(&self, mut para: usize, mut offset: usize) -> Rect {
    let visual_lines = &self.visual_info.visual_lines;
    if visual_lines.is_empty() {
      return Rect::zero();
    }
    if para >= visual_lines.len() {
      para = visual_lines.len() - 1;
      offset = visual_lines[para].glyphs.len();
    }

    let line = &visual_lines[para];
    let glyph = line.glyphs.get(offset);
    let line_dir = self.visual_info.line_dir;

    glyph.map_or_else(
      || match line_dir.is_horizontal() {
        true => Rect::new(
          Point::new(
            self.to_pixel_value(line.x),
            self.to_pixel_value(line.y + line.height),
          ),
          Size::new(self.to_pixel_value(line.width), 0.),
        ),
        false => Rect::new(
          Point::new(
            self.to_pixel_value(line.width + line.x),
            self.to_pixel_value(line.y),
          ),
          Size::new(0., self.to_pixel_value(line.height)),
        ),
      },
      |glyph| {
        let orign = Point::new(
          self.to_pixel_value(glyph.x_offset + line.x),
          self.to_pixel_value(glyph.y_offset + line.y),
        );
        let size = match line_dir.is_horizontal() {
          true => Size::new(
            self.to_pixel_value(line.width),
            self.to_pixel_value(glyph.y_advance),
          ),
          false => Size::new(
            self.to_pixel_value(glyph.x_advance),
            self.to_pixel_value(line.height),
          ),
        };
        Rect::new(orign, size)
      },
    )
  }

  pub fn line_height(&self, para: usize) -> f32 {
    self
      .visual_info
      .visual_lines
      .get(para)
      .map_or(0., |line| self.to_pixel_value(line.height))
  }

  pub fn select_range(&self, rg: &Range<usize>) -> Vec<Rect> {
    struct TypoRectJointer
    {
      acc: Vec<Rect>,
      cur: Option<Rect>,
    }

    impl TypoRectJointer
    {
      fn new() -> Self { Self { acc: vec![], cur: None } }
      fn join_x(&mut self, next: Rect) {
        if let Some(rc) = &mut self.cur {
          rc.size.width = next.max_x() - rc.min_x();
        } else {
          self.cur = Some(next);
        }
      }
      fn new_rect(&mut self) {
        let cur = self.cur.take();
        if let Some(rc) = cur {
          self.acc.push(rc);
        }
      }
      fn rects(self) -> Vec<Rect> { self.acc }
    }
    let mut jointer = TypoRectJointer::new();
    for line in &self.visual_info.visual_lines {
      let height = (line.height * self.scale).into();
      let offset_x = self.to_pixel_value(line.x).into();
      let offset_y = self.to_pixel_value(line.y).into();
      for glyph in &line.glyphs {
        if rg.contains(&(glyph.cluster as usize)) {
          let glyph = self.scale_to_pixel_glyph(glyph);
          let rc = Rect::new(
            Point::new(glyph.x_offset + offset_x, glyph.y_offset + offset_y),
            Size::new(glyph.x_advance, height),
          );
          jointer.join_x(rc);
        } else {
          jointer.new_rect();
        }
      }
      jointer.new_rect();
    }
    jointer.rects()
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
      .flat_map(move |l| {
        l.glyphs.iter().map(|g| {
          let mut g = g.clone();
          g.x_offset += l.x;
          g.y_offset += l.y;
          g
        })
      })
      .filter(|g| {
        Em::zero() <= g.x_offset + g.x_advance
          && g.x_offset < self.bounds.width
          && Em::zero() <= g.y_offset + g.y_advance
          && g.y_offset < self.bounds.height
      })
      .map(move |g| self.scale_to_pixel_glyph(&g))
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
    let text = "Hello
    
    
    
    world!"
      .into();

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

    assert_eq!(visual.visual_rect().size, Size::new(61.960938, 70.));
  }

  #[test]
  fn empty_text_bounds() {
    let text = "".into();

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

    assert_eq!(visual.visual_rect().size, Size::new(0., 14.0));
  }

  #[test]
  fn new_line_bounds() {
    let text = "123\n".into();

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

    assert_eq!(visual.visual_rect().size, Size::new(35.123047, 28.));
  }

  #[test]
  fn simple_typography_text() {
    fn glyphs(cfg: TypographyCfg) -> Vec<(f32, f32)> {
      let text = "Hello--------\nworld!".into();
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
        (0.0, 10.),
        (8.303711, 10.),
        (14.546876, 10.),
        (18.783205, 10.),
        (21.686525, 10.),
        (28.159182, 10.)
      ]
    );

    cfg.text_align = Some(TextAlign::End);
    cfg.bounds.width = Pixel(100.0.into()).into();
    let r_align = glyphs(cfg.clone());
    assert_eq!(
      &r_align,
      &[
        (37.849617, 0.0),
        (45.49415, 0.0),
        (51.771492, 0.0),
        (54.674816, 0.0),
        (57.578133, 0.0),
        (64.00684, 0.0),
        (67.74024, 0.0),
        (71.47365, 0.0),
        (75.20705, 0.0),
        (78.94044, 0.0),
        (82.673836, 0.0),
        (86.407234, 0.0),
        (90.140625, 0.0),
        (93.87402, 0.0),
        // second line
        (67.70703, 10.0),
        (76.010735, 10.0),
        (82.25391, 10.0),
        (86.490234, 10.0),
        (89.393555, 10.0),
        (95.86621, 10.0)
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
        (0.0, 80.),
        (8.303711, 80.),
        (14.546876, 80.),
        (18.783205, 80.),
        (21.686525, 80.),
        (28.159182, 80.),
        // second line
        (0.0, 90.),
        (7.6445313, 90.),
        (13.921876, 90.),
        (16.825197, 90.),
        (19.728518, 90.),
        (26.157228, 90.),
        (29.890629, 90.),
        (33.624027, 90.),
        (37.357426, 90.),
        (41.09082, 90.),
        (44.82422, 90.),
        (48.557617, 90.),
        (52.29101, 90.),
        (56.024406, 90.)
      ],
    );

    cfg.text_align = Some(TextAlign::Center);
    cfg.line_dir = PlaceLineDirection::TopToBottom;
    cfg.bounds = Size::new(Pixel::from(40.).into(), Pixel::from(15.).into());
    let center_clip = glyphs(cfg);
    assert_eq!(
      &center_clip,
      &[
        (-3.4306602, 0.0),
        (2.8466845, 0.0),
        (5.7500052, 0.0),
        (8.653326, 0.0),
        (15.082037, 0.0),
        (18.815437, 0.0),
        (22.548836, 0.0),
        (26.282234, 0.0),
        (30.01563, 0.0),
        (33.749027, 0.0),
        (37.48242, 0.0),
        (3.8535142, 10.0),
        (12.157226, 10.0),
        (18.40039, 10.0),
        (22.636719, 10.0),
        (25.54004, 10.0),
        (32.012695, 10.0)
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
    let text: Substr = "hi!".into();
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

  #[test]
  fn typo_cluster_test() {
    let cfg = TypographyCfg {
      line_height: None,
      letter_space: None,
      text_align: None,
      bounds: (Em::MAX, Em::MAX).into(),
      line_dir: PlaceLineDirection::TopToBottom,
      overflow: Overflow::Clip,
    };
    let text =
      "abcd \u{202e} right_to_left_1 \u{202d} embed \u{202c} right_to_left_2 \u{202c} end".into();
    let graphys = typography_text(text, FontSize::Em(Em::absolute(1.0)), cfg);
    assert!((0, 4) == graphys.position_by_cluster(4));
    assert!((0, 35) == graphys.position_by_cluster(22));
    assert!((0, 27) == graphys.position_by_cluster(31));
    assert!((0, 8) == graphys.position_by_cluster(53));
  }
}
