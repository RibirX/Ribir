use std::{cell::RefCell, ops::Range};

use font_db::GlyphBaseline;
use ribir_algo::{FrameCache, Sc, Substr};
use ribir_geom::{Point, Rect, Size};

use crate::{
  font_db::FontDB,
  shaper::{NEWLINE_GLYPH_ID, TextShaper},
  text_reorder::ReorderResult,
  typography::*,
  *,
};

#[derive(Clone, PartialEq, Eq, Hash)]
struct RunKey {
  pub ids: Box<[ID]>,
  pub line_height: GlyphUnit,
  pub letter_space: GlyphUnit,
  pub text: Substr,
}

/// Typography `text` relative to 1em.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypographyKey {
  runs: Box<[RunKey]>,
  /// The maximum width of a line can be utilized to position glyphs, enabling
  /// us to determine if a cache can be reused even if its bounds are different.
  line_width: GlyphUnit,
  text_align: TextAlign,
  baseline: GlyphBaseline,
  line_dir: PlaceLineDirection,
  overflow: TextOverflow,
}

/// Do simple text typography and cache it.
pub struct TypographyStore {
  reorder: TextReorder,
  shaper: TextShaper,
  font_db: Sc<RefCell<FontDB>>,
  cache: FrameCache<TypographyKey, Sc<VisualInfos>>,
}

#[derive(Clone)]
pub struct VisualGlyphs {
  font_size: f32,
  x: GlyphUnit,
  y: GlyphUnit,
  visual_info: Sc<VisualInfos>,
  order_info: Sc<ReorderResult>,
}

impl VisualGlyphs {
  pub fn new(font_size: f32, order_info: Sc<ReorderResult>, visual_info: Sc<VisualInfos>) -> Self {
    Self { font_size, visual_info, order_info, x: GlyphUnit::ZERO, y: GlyphUnit::ZERO }
  }

  pub fn align(&mut self, display_bounds: Rect) {
    let display_bounds = display_bounds / self.font_size * GlyphUnit::PIXELS_PER_EM as f32;
    self.x = GlyphUnit::from_pixel(display_bounds.min_x());
    self.y = GlyphUnit::from_pixel(display_bounds.min_y());
    let bound_width = GlyphUnit::from_pixel(display_bounds.width());
    let bound_height = GlyphUnit::from_pixel(display_bounds.height());
    let VisualInfos { text_align, line_dir, visual_size, .. } = *self.visual_info;

    if line_dir.is_horizontal() {
      self.y += text_align_offset(visual_size.height, bound_height, text_align);
    } else {
      self.x += text_align_offset(visual_size.width, bound_width, text_align);
    }

    if line_dir == PlaceLineDirection::RightToLeft {
      self.x += bound_width - visual_size.width;
    }
    if line_dir == PlaceLineDirection::BottomToTop {
      self.y += bound_height - visual_size.height;
    }
  }

  pub fn font_size(&self) -> f32 { self.font_size }
}

impl TypographyStore {
  pub fn new(font_db: Sc<RefCell<FontDB>>) -> Self {
    let reorder = TextReorder::default();
    let shaper = TextShaper::new(font_db.clone());
    TypographyStore { reorder, shaper, font_db, cache: <_>::default() }
  }

  pub fn end_frame(&mut self) {
    self.reorder.end_frame();
    self.shaper.end_frame();
    self.cache.end_frame("Typography");
  }

  /// Do a simply typography that only support single style.
  pub fn typography(
    &mut self, text: Substr, style: &TextStyle, bounds: Size, text_align: TextAlign,
    baseline: GlyphBaseline, line_dir: PlaceLineDirection,
  ) -> VisualGlyphs {
    let TextStyle { font_size, ref font_face, letter_space, line_height, overflow } = *style;
    // Since we cache the result of the standard font size, we must ensure that all
    // variables are cast relative to this standard font size.
    let scale = font_size / GlyphUnit::PIXELS_PER_EM as f32;
    let bounds = Size::new(
      GlyphUnit::from_pixel(bounds.width / scale),
      GlyphUnit::from_pixel(bounds.height / scale),
    );
    let letter_space =
      GlyphUnit::from_pixel(letter_space / font_size * GlyphUnit::PIXELS_PER_EM as f32);
    let line_height =
      GlyphUnit::from_pixel(line_height / font_size * GlyphUnit::PIXELS_PER_EM as f32);

    let info = self.reorder.reorder_text(&text).clone();
    let ids = self
      .font_db
      .borrow_mut()
      .select_all_match(font_face)
      .into_boxed_slice();
    let runs = [RunKey { ids, line_height, letter_space, text }].into();
    let key = TypographyKey::new(runs, bounds, text_align, line_dir, overflow, baseline);
    let infos = if let Some(infos) = self.cache.get(&key).cloned() {
      infos
    } else {
      let ids = &key.runs[0].ids;
      let text = &key.runs[0].text;
      let inputs = info.paras.iter().map(|p| {
        p.runs
          .iter()
          .map(|r| {
            let dir = if r.is_empty() || p.levels[r.start].is_ltr() {
              TextDirection::LeftToRight
            } else {
              TextDirection::RightToLeft
            };

            let shape_result = self
              .shaper
              .shape_text(&text.substr(r.clone()), ids, dir, baseline);
            InputRun::new(shape_result, 1., letter_space, r.clone())
          })
          .collect()
      });

      let t_man = TypographyMan::new(inputs, line_dir, text_align, line_height, bounds, overflow);
      let visual_info = t_man.typography_all();
      let infos = Sc::new(visual_info);
      self.cache.put(key, infos.clone());
      infos
    };

    VisualGlyphs::new(font_size, info, infos.clone())
  }

  pub fn font_db(&self) -> &Sc<RefCell<FontDB>> { &self.font_db }
}

impl VisualGlyphs {
  /// return a visual rect to place the text in pixel.
  pub fn visual_rect(&self) -> Rect {
    let info = &self.visual_info;

    Rect::new(
      Point::new(self.to_pixel_value(self.x), self.to_pixel_value(self.y)),
      Size::new(
        self.to_pixel_value(info.visual_size.width),
        self.to_pixel_value(info.visual_size.height),
      ),
    )
  }

  pub fn nearest_glyph(&self, offset_x: f32, offset_y: f32) -> (usize, usize) {
    let scale = self.font_size / GlyphUnit::PIXELS_PER_EM as f32;
    let x = GlyphUnit::from_pixel(offset_x / scale) - self.x;
    let y = GlyphUnit::from_pixel(offset_y / scale) - self.y;
    let mut bottom = self.visual_info.visual_size.height;

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
        .find(|(_, g)| GlyphUnit::ZERO < g.x_advance && g.x_offset <= x)
        .map(|(i, _)| i)
        .unwrap_or(0);
      return (row, idx);
    }

    (0, 0)
  }

  pub fn position_by_cluster(&self, cluster: usize) -> (usize, usize) {
    struct RangeLocator<'a> {
      ranges: Vec<(&'a Range<usize>, usize)>,
    }

    impl<'a> RangeLocator<'a> {
      fn from_unorder_ranges(rgs: impl Iterator<Item = &'a Range<usize>>) -> Self {
        let mut ranges: Vec<_> = rgs
          .enumerate()
          .map(|(idx, item)| (item, idx))
          .collect();
        ranges.sort_by(|lh, rh| lh.0.start.cmp(&rh.0.start));
        RangeLocator { ranges }
      }

      fn range_index(&self, val: usize) -> Option<usize> {
        let idx = self
          .ranges
          .partition_point(|item| item.0.end <= val);
        if idx < self.ranges.len() && self.ranges[idx].0.contains(&val) {
          Some(self.ranges[idx].1)
        } else {
          None
        }
      }
    }

    let visual_lines = &self.visual_info.visual_lines;
    if visual_lines.is_empty() {
      return (0, 0);
    }

    let para = self
      .order_info
      .paras
      .partition_point(|p| p.range.end <= cluster)
      .min(self.order_info.paras.len() - 1);

    let order_info = &self.order_info.paras[para];
    let locator = RangeLocator::from_unorder_ranges(order_info.runs.iter());
    let dst_run = locator.range_index(cluster);
    let is_ltr = dst_run.is_none_or(|run| order_info.levels[order_info.runs[run].start].is_ltr());
    let is_layout_before = |glyph_cluster: usize| {
      if dst_run.is_none() {
        return true;
      }
      if glyph_cluster < order_info.range.start {
        return true;
      } else if order_info.range.end <= glyph_cluster {
        return false;
      }
      let glyph_run = locator.range_index(glyph_cluster).unwrap();
      let dst_run = dst_run.unwrap();
      if dst_run == glyph_run {
        if is_ltr {
          return glyph_cluster < cluster;
        } else {
          return glyph_cluster > cluster;
        }
      }
      glyph_run < dst_run
    };
    let line_para = visual_lines.partition_point(|l| {
      if l.glyphs.is_empty() {
        return false;
      }
      is_layout_before(l.glyphs.first().map(|g| g.cluster).unwrap() as usize)
        && is_layout_before(l.glyphs.last().map(|g| g.cluster).unwrap() as usize)
    });
    if line_para >= visual_lines.len() {
      return (visual_lines.len() - 1, visual_lines.last().unwrap().glyphs.len());
    }
    let line = &visual_lines[line_para];
    let offset = line
      .glyphs
      .partition_point(|glyph| is_layout_before(glyph.cluster as usize));
    (line_para, offset)
  }

  pub fn position_to_cluster(&self, row: usize, col: usize) -> usize {
    let lines = &self.visual_info.visual_lines;

    if row < lines.len() && col < lines[row].glyphs.len() {
      lines[row].glyphs[col].cluster as usize
    } else {
      lines
        .get(row + 1)
        .and_then(|l| l.glyphs.first().map(|g| g.cluster as usize))
        .unwrap_or_else(|| {
          self
            .order_info
            .paras
            .last()
            .map_or(0, |p| p.range.end)
        })
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

    let mut rc = glyph.map_or_else(
      || match line_dir.is_horizontal() {
        true => Rect::new(
          Point::new(self.to_pixel_value(line.x), self.to_pixel_value(line.y + line.height)),
          Size::new(self.to_pixel_value(line.width), 0.),
        ),
        false => Rect::new(
          Point::new(self.to_pixel_value(line.width + line.x), self.to_pixel_value(line.y)),
          Size::new(0., self.to_pixel_value(line.height)),
        ),
      },
      |glyph| {
        let origin = Point::new(
          self.to_pixel_value(glyph.x_offset + line.x),
          self.to_pixel_value(glyph.y_offset + line.y),
        );
        let size = match line_dir.is_horizontal() {
          true => Size::new(self.to_pixel_value(line.width), self.to_pixel_value(glyph.y_advance)),
          false => {
            Size::new(self.to_pixel_value(glyph.x_advance), self.to_pixel_value(line.height))
          }
        };
        Rect::new(origin, size)
      },
    );
    rc.origin += Point::new(self.to_pixel_value(self.x), self.to_pixel_value(self.y)).to_vector();
    rc
  }

  pub fn line_height(&self, para: usize) -> f32 {
    self
      .visual_info
      .visual_lines
      .get(para)
      .map_or(0., |line| self.to_pixel_value(line.height))
  }

  pub fn select_range(&self, rg: &Range<usize>) -> Vec<Rect> {
    let mut rects = vec![];
    for line in &self.visual_info.visual_lines {
      let mut inline: Vec<(usize, usize)> = vec![];
      for (i, glyph) in line.glyphs.iter().enumerate() {
        if rg.contains(&(glyph.cluster as usize)) {
          if let Some((_, end)) = inline
            .last_mut()
            .filter(|(_, end)| (*end + 1) == i)
          {
            *end += 1;
          } else {
            inline.push((i, i));
          }
        }
      }

      if !inline.is_empty() {
        let is_horizontal = !self.visual_info.line_dir.is_horizontal();
        let offset_x = self.x + line.x;
        let offset_y = self.y + line.y;
        inline.into_iter().for_each(|(start, end)| {
          let (x_min, x_max, y_min, y_max) = if is_horizontal {
            (
              line.glyphs[start].x_offset,
              line.glyphs[end].x_offset + line.glyphs[end].x_advance,
              GlyphUnit::ZERO,
              line.height,
            )
          } else {
            (
              GlyphUnit::ZERO,
              line.width,
              line.glyphs[start].y_offset,
              line.glyphs[end].y_offset + line.glyphs[end].y_advance,
            )
          };
          let rect = Rect::from_points([
            Point::new(self.to_pixel_value(x_min), self.to_pixel_value(y_min)),
            Point::new(self.to_pixel_value(x_max), self.to_pixel_value(y_max)),
          ]);
          rects.push(
            rect.translate((self.to_pixel_value(offset_x), self.to_pixel_value(offset_y)).into()),
          );
        });
      }
    }
    rects
  }

  fn to_pixel_value(&self, v: GlyphUnit) -> f32 { v.cast_to(self.font_size).into_pixel() }

  pub fn glyphs(&self) -> impl Iterator<Item = Glyph> + '_ {
    let hor_line = self.is_horizontal_line();
    self
      .visual_info
      .visual_lines
      .iter()
      .flat_map(move |l| l.glyphs_iter(hor_line))
      .map(move |g| g.cast_to(self.font_size))
  }

  pub fn glyphs_in_bounds(&self, rc: &Rect) -> Option<impl Iterator<Item = Glyph> + '_> {
    let visual_rect = self.visual_rect();
    let mut rc = visual_rect.intersection(rc)?;
    rc.origin -= visual_rect.origin.to_vector();

    let scale = self.font_size / GlyphUnit::PIXELS_PER_EM as f32;
    let min_x = GlyphUnit::from_pixel(rc.min_x() / scale);
    let min_y = GlyphUnit::from_pixel(rc.min_y() / scale);
    let max_x = GlyphUnit::from_pixel(rc.max_x() / scale);
    let max_y = GlyphUnit::from_pixel(rc.max_y() / scale);

    let hor_line = self.is_horizontal_line();
    let iter = self
      .visual_info
      .visual_lines
      .iter()
      .filter(move |l| {
        min_x < l.x + l.width && l.x < max_x && min_y < l.y + l.height && l.y < max_y
      })
      .flat_map(move |l| l.glyphs_iter(hor_line))
      .filter(move |g| {
        min_x < g.x_offset + g.x_advance
          && g.x_offset < max_x
          && min_y < g.y_offset + g.y_advance
          && g.y_offset < max_y
      })
      .map(move |g| g.cast_to(self.font_size));
    Some(iter)
  }

  pub fn glyph_count(&self, row: usize, ignore_new_line: bool) -> usize {
    self
      .visual_info
      .visual_lines
      .get(row)
      .map_or(0, |l| {
        if ignore_new_line {
          if l
            .glyphs
            .last()
            .is_some_and(|g| g.glyph_id == NEWLINE_GLYPH_ID)
          {
            l.glyphs.len() - 1
          } else {
            l.glyphs.len()
          }
        } else {
          l.glyphs.len()
        }
      })
  }

  pub fn glyph_row_count(&self) -> usize { self.visual_info.visual_lines.len() }

  fn is_horizontal_line(&self) -> bool { !self.visual_info.line_dir.is_horizontal() }
}

impl TypographyKey {
  fn new(
    runs: Box<[RunKey]>, bounds: Size<GlyphUnit>, text_align: TextAlign,
    line_dir: PlaceLineDirection, overflow: TextOverflow, baseline: GlyphBaseline,
  ) -> Self {
    let line_width = match overflow {
      // line width is not so important in clip mode, the cache can be use even with difference line
      // width. The wider one can use for the narrower one. S
      TextOverflow::Overflow => GlyphUnit::MAX,

      TextOverflow::AutoWrap => {
        if line_dir.is_horizontal() {
          bounds.height
        } else {
          bounds.width
        }
      }
    };

    Self { runs, line_width, text_align, line_dir, overflow, baseline }
  }
}

#[cfg(test)]
mod tests {
  use core::f32;

  use lyon_algorithms::geom::euclid::Rect;

  use super::*;
  use crate::FontFamily;

  fn test_store() -> TypographyStore {
    let font_db = Sc::new(RefCell::new(FontDB::default()));
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = font_db.borrow_mut().load_font_file(path);
    TypographyStore::new(font_db)
  }

  fn test_face() -> FontFace {
    FontFace { families: Box::new([FontFamily::Name("DejaVu Sans".into())]), ..<_>::default() }
  }
  fn text_style(font_size: f32, overflow: TextOverflow, letter_space: f32) -> TextStyle {
    TextStyle { font_size, font_face: test_face(), letter_space, line_height: font_size, overflow }
  }
  fn zero_letter_space_style(font_size: f32, overflow: TextOverflow) -> TextStyle {
    text_style(font_size, overflow, 0.)
  }

  fn typography_text(
    text: Substr, style: &TextStyle, bounds: Size, text_align: TextAlign,
    line_dir: PlaceLineDirection,
  ) -> VisualGlyphs {
    let mut store = test_store();
    store.typography(text, style, bounds, text_align, GlyphBaseline::Alphabetic, line_dir)
  }

  #[test]
  fn simple_text_bounds() {
    let text = "Hello
    
    
    
    world!"
      .into();

    let style = zero_letter_space_style(14., TextOverflow::Overflow);
    let visual = typography_text(
      text,
      &style,
      (f32::MAX, f32::MAX).into(),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(visual.visual_rect().size, Size::new(61.960938, 70.));
  }

  #[test]
  fn empty_text_bounds() {
    let text = "".into();

    let style = zero_letter_space_style(14., TextOverflow::Overflow);
    let visual = typography_text(
      text,
      &style,
      (f32::MAX, f32::MAX).into(),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(visual.visual_rect().size, Size::new(0., 14.0));
  }

  #[test]
  fn new_line_bounds() {
    let text = "123\n".into();
    let style = zero_letter_space_style(14., TextOverflow::Overflow);
    let visual = typography_text(
      text,
      &style,
      (f32::MAX, f32::MAX).into(),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(visual.visual_rect().size, Size::new(34.164063, 28.));
  }

  #[test]
  fn simple_typography_text() {
    fn glyphs(
      overflow: TextOverflow, bounds: Size, text_align: TextAlign, line_dir: PlaceLineDirection,
    ) -> Vec<(f32, f32)> {
      let text = "Hello--------\nworld!".into();
      let style = text_style(10., overflow, 2.);

      let mut info = typography_text(text, &style, bounds, text_align, line_dir);
      info.align(Rect::from_size(bounds));

      let visual_rc = info.visual_rect();
      info
        .glyphs_in_bounds(&Rect::from_size(bounds))
        .unwrap()
        .map(|g| {
          let bounds = g.bounds();
          (visual_rc.origin.x + bounds.min_x(), visual_rc.origin.y + bounds.min_y())
        })
        .collect()
    }

    let not_bounds = glyphs(
      TextOverflow::Overflow,
      Size::new(f32::MAX, f32::MAX),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );
    assert_eq!(
      &not_bounds,
      &[
        (0.0, 0.0),
        (9.520508, 0.0),
        (17.672852, 0.0),
        (22.451172, 0.0),
        (27.229492, 0.0),
        (35.533203, 0.0),
        (41.1416, 0.0),
        (46.75, 0.0),
        (52.3584, 0.0),
        (57.967773, 0.0),
        (63.57617, 0.0),
        (69.18457, 0.0),
        (74.79297, 0.0),
        (80.40137, 0.0),
        // second line
        (0.0, 10.0),
        (10.1796875, 10.0),
        (18.297852, 10.0),
        (24.40918, 10.0),
        (29.1875, 10.0),
        (37.535156, 10.0)
      ]
    );

    let r_align = glyphs(
      TextOverflow::Overflow,
      Size::new(100., f32::MAX),
      TextAlign::End,
      PlaceLineDirection::TopToBottom,
    );
    assert_eq!(
      &r_align,
      &[
        (12.28418, 0.0),
        (21.804688, 0.0),
        (29.957031, 0.0),
        (34.73535, 0.0),
        (39.51367, 0.0),
        (47.817383, 0.0),
        (53.42578, 0.0),
        (59.03418, 0.0),
        (64.64258, 0.0),
        (70.25195, 0.0),
        (75.86035, 0.0),
        (81.46875, 0.0),
        (87.07715, 0.0),
        (92.68555, 0.0),
        // second line
        (56.458008, 10.0),
        (66.63672, 10.0),
        (74.75488, 10.0),
        (80.86621, 10.0),
        (85.64453, 10.0),
        (93.99219, 10.0)
      ],
    );

    let bottom = glyphs(
      TextOverflow::Overflow,
      Size::new(100., 100.),
      TextAlign::Start,
      PlaceLineDirection::BottomToTop,
    );

    assert_eq!(
      &bottom,
      &[
        // first line
        (0.0, 90.),
        (10.1796875, 90.),
        (18.297852, 90.),
        (24.40918, 90.),
        (29.1875, 90.),
        (37.535156, 90.),
        // second line
        (0.0, 80.),
        (9.520508, 80.),
        (17.672852, 80.),
        (22.451172, 80.),
        (27.229492, 80.),
        (35.533203, 80.),
        (41.1416, 80.),
        (46.75, 80.),
        (52.3584, 80.),
        (57.967773, 80.),
        (63.57617, 80.),
        (69.18457, 80.),
        (74.79297, 80.),
        (80.40137, 80.)
      ],
    );

    let center_clip = glyphs(
      TextOverflow::Overflow,
      Size::new(40., 15.),
      TextAlign::Center,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(
      &center_clip,
      &[
        (-1.40625, 0.0),
        (3.3720703, 0.0),
        (11.675781, 0.0),
        (17.28418, 0.0),
        (22.892578, 0.0),
        (28.500977, 0.0),
        (34.11035, 0.0),
        (39.71875, 0.0),
        (-1.7705078, 10.0),
        (8.408203, 10.0),
        (16.527344, 10.0),
        (22.638672, 10.0),
        (27.416992, 10.0),
        (35.76465, 10.0)
      ],
    );
  }

  #[test]
  fn cache_test() {
    let mut store = test_store();

    let text: Substr = "hi!".into();

    let style = zero_letter_space_style(16., TextOverflow::Overflow);

    assert!(store.cache.is_empty());

    let visual = store.typography(
      text.clone(),
      &style,
      Size::new(f32::MAX, f32::MAX),
      TextAlign::Start,
      GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(visual.glyphs().count(), 3);

    assert_eq!(store.cache.len(), 1);

    store.typography(
      text.clone(),
      &style,
      Size::new(f32::MAX, f32::MAX),
      TextAlign::Start,
      GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );

    assert_eq!(store.cache.len(), 1);

    store.end_frame();
    store.end_frame();

    assert!(store.cache.is_empty());
  }

  #[test]
  fn cluster_position() {
    let style = zero_letter_space_style(15., TextOverflow::Overflow);
    let text =
      "abcd \u{202e} right_to_left_1 \u{202d} embed \u{202c} right_to_left_2 \u{202c} end".into();
    let glyphs = typography_text(
      text,
      &style,
      Size::new(f32::MAX, f32::MAX),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );

    assert!((0, 4) == glyphs.position_by_cluster(4));
    assert!((0, 35) == glyphs.position_by_cluster(22));
    assert!((0, 27) == glyphs.position_by_cluster(31));
    assert!((0, 8) == glyphs.position_by_cluster(53));
  }

  #[test]
  fn auto_wrap_position() {
    let style = zero_letter_space_style(16., TextOverflow::AutoWrap);
    let text = "WITHIN BOUND\rLINE WITH LONG WORD LIKE: ABCDEFGHIJKLMNOPQRSTUVWXYZ, WILL AUTO \
                WRAP TO 3 LINES."
      .into();
    let glyphs = typography_text(
      text,
      &style,
      Size::new(GlyphUnit::PIXELS_PER_EM as f32 * 14.0, GlyphUnit::PIXELS_PER_EM as f32 * 2.0),
      TextAlign::Start,
      PlaceLineDirection::TopToBottom,
    );

    // text will auto wrap layout to 5 line as follow:
    let line1 = "WITHIN BOUND\r";
    let line2 = "LINE WITH LONG WORD ";
    let line3 = "LIKE: ";
    let line4 = "ABCDEFGHIJKLMNOPQRSTU";
    let line5 = "VWXYZ, WILL AUTO WRAP ";
    let _line6 = "TO 3 LINES.";

    // check auto wrap
    assert!((1, 0) == glyphs.position_by_cluster(line1.len()));
    assert!((2, 0) == glyphs.position_by_cluster(line1.len() + line2.len()));
    assert!((3, 0) == glyphs.position_by_cluster(line1.len() + line2.len() + line3.len()));
    assert!(
      (4, 0) == glyphs.position_by_cluster(line1.len() + line2.len() + line3.len() + line4.len())
    );
    assert!(
      (5, 0)
        == glyphs
          .position_by_cluster(line1.len() + line2.len() + line3.len() + line4.len() + line5.len())
    );
  }

  #[test]
  fn text_in_different_bounds() {
    let mut store = test_store();
    let text: Substr = "1234".into();

    let style = zero_letter_space_style(16., TextOverflow::Overflow);
    let bounds1 =
      Size::new(10. * GlyphUnit::PIXELS_PER_EM as f32, 2. * GlyphUnit::PIXELS_PER_EM as f32);
    let mut glyphs1 = store.typography(
      text.clone(),
      &style,
      bounds1,
      TextAlign::Center,
      GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );
    glyphs1.align(Rect::from_size(bounds1));

    let bounds2 =
      Size::new(20. * GlyphUnit::PIXELS_PER_EM as f32, 2. * GlyphUnit::PIXELS_PER_EM as f32);
    let mut glyphs2 = store.typography(
      text,
      &style,
      bounds2,
      TextAlign::Center,
      GlyphBaseline::Alphabetic,
      PlaceLineDirection::TopToBottom,
    );
    glyphs2.align(Rect::from_size(bounds2));

    let offset_x = 5. * GlyphUnit::PIXELS_PER_EM as f32;
    assert_eq!(
      glyphs2.visual_rect().origin - glyphs1.visual_rect().origin,
      ribir_geom::Vector::new(offset_x, 0.)
    );
    assert_eq!(1, store.cache.len());
  }
}
