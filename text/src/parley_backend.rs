use std::{
  rc::Rc,
  sync::{OnceLock, RwLock},
};

use parley::{
  FontContext as ParleyFontContext, FontData as ParleyFontData, Layout as ParleyLayout,
  LayoutContext as ParleyLayoutContext,
  editing::{Cursor as ParleyCursor, Selection as ParleySelection},
  fontique::{Blob, CollectionOptions},
  layout::{Affinity as ParleyAffinity, BreakReason, PositionedLayoutItem},
  style::{
    FontFamily as ParleyFontFamily, FontFamilyName as ParleyFontFamilyName,
    FontStyle as ParleyFontStyle, FontWeight as ParleyFontWeight, FontWidth as ParleyFontWidth,
    GenericFamily, LineHeight as ParleyLineHeight, OverflowWrap as ParleyOverflowWrap,
    StyleProperty, TextWrapMode,
  },
};
use ribir_algo::Arc;
use ribir_types::{BoxClamp, Point, Rect, Size, Vector};
use swash::FontRef;

use crate::{
  AttributedText, FontSystem,
  font::{FontFaceId, FontFaceMetrics, FontFamily, FontLoadError, FontStretch, FontStyle},
  paint::{DrawGlyph, DrawGlyphRun, DrawTextDecoration, GlyphId, TextDrawPayload},
  paragraph::{
    Caret, CaretAffinity, CaretMotion, ClusterIndex, LineIndex, Paragraph, ParagraphLayout,
    ParagraphLayoutRef, TextByteIndex, TextHitResult, TextRange, TextSpan, VisualPosition,
  },
  raster::{GlyphRasterSource, GlyphRasterSourceRef, RasterBitmap, RasterBitmapFormat},
  style::{Color, LineHeight, ParagraphStyle, TextAlign, TextDecoration, TextStyle, TextWrap},
};

fn build_text_paragraph<Brush>(
  source: AttributedText<Brush>, engine: Rc<std::cell::RefCell<ParleyEngine>>, faces: ParleyFaces,
) -> Rc<dyn Paragraph<Brush>>
where
  Brush: Clone + From<Color> + PartialEq + 'static,
{
  Rc::new(ParleyParagraph { source, engine, faces })
}

fn build_text_raster_source(faces: ParleyFaces) -> GlyphRasterSourceRef {
  Arc::new(Box::new(ParleyGlyphRasterSource { faces }))
}

struct ParleyParagraph<Brush> {
  source: AttributedText<Brush>,
  engine: Rc<std::cell::RefCell<ParleyEngine>>,
  faces: ParleyFaces,
}

struct ParleyParagraphLayout<Brush> {
  layout: Arc<ParleyLayout<usize>>,
  logical_size: Size,
  payload: TextDrawPayload<Brush>,
  line_offsets: Box<[f32]>,
  line_positions: OnceLock<Box<[VisualLine]>>,
}

use crate::svg_glyph::extract_svg_glyph;

type ParleyFaces = Arc<RwLock<std::collections::HashMap<FontFaceId, ParleyFace>>>;

#[derive(Clone, Default)]
struct ParleyGlyphRasterSource {
  faces: ParleyFaces,
}

#[derive(Clone)]
struct ParleyFace {
  font: ParleyFontData,
  metrics: FontFaceMetrics,
}

struct ParleyEngine {
  font_ctx: ParleyFontContext,
  layout_ctx: ParleyLayoutContext<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CursorKey {
  index: usize,
  affinity: ParleyAffinity,
}

#[derive(Clone, Copy, Debug)]
struct VisualSlot {
  key: CursorKey,
  x: f32,
}

#[derive(Debug)]
struct VisualLine {
  top: f32,
  height: f32,
  slots: Box<[VisualSlot]>,
}

impl CursorKey {
  fn from_cursor(cursor: ParleyCursor) -> Self {
    Self { index: cursor.index(), affinity: cursor.affinity() }
  }
}

impl GlyphRasterSource for ParleyGlyphRasterSource {
  fn face_metrics(&self, face_id: FontFaceId) -> Option<FontFaceMetrics> {
    self
      .faces
      .read()
      .unwrap()
      .get(&face_id)
      .map(|face| face.metrics)
  }

  fn raster_bitmap(
    &self, face_id: FontFaceId, glyph_id: GlyphId, font_ppem: u16,
  ) -> Option<RasterBitmap> {
    let face = self
      .faces
      .read()
      .unwrap()
      .get(&face_id)
      .cloned()?;
    raster_bitmap_from_font(&face.font, glyph_id, font_ppem)
  }

  fn raster_svg(&self, face_id: FontFaceId, glyph_id: GlyphId) -> Option<String> {
    let faces = self.faces.read().ok()?;
    let face = faces.get(&face_id)?;
    let font_ref = swash::FontRef::from_index(face.font.data.data(), face.font.index as usize)?;
    extract_svg_glyph(glyph_id, &font_ref, face.font.index)
  }
}

impl ParleyEngine {
  fn new() -> Self {
    let mut font_ctx = ParleyFontContext::new();
    font_ctx.collection = parley::fontique::Collection::new(CollectionOptions {
      shared: false,
      system_fonts: !cfg!(target_arch = "wasm32"),
    });

    Self { font_ctx, layout_ctx: ParleyLayoutContext::new() }
  }

  pub fn register_font_bytes(&mut self, data: Vec<u8>) {
    let blob = Blob::from(data);
    self
      .font_ctx
      .collection
      .register_fonts(blob, None);
  }

  fn register_face(font: &ParleyFontData, faces: &ParleyFaces) -> (FontFaceId, FontFaceMetrics) {
    let face_id = FontFaceId::new(font.data.id(), font.index);
    let metrics = font_metrics(font);
    let mut faces = faces.write().unwrap();
    if let std::collections::hash_map::Entry::Vacant(entry) = faces.entry(face_id) {
      entry.insert(ParleyFace { metrics, font: font.clone() });
    }
    (face_id, metrics)
  }

  fn intrinsic_widths<Brush>(
    &mut self, source: &AttributedText<Brush>, text_style: &TextStyle,
    paragraph_style: &ParagraphStyle,
  ) -> (f32, f32)
  where
    Brush: Clone + From<Color>,
  {
    let text = source.text.as_ref();
    let mut builder = self
      .layout_ctx
      .ranged_builder(&mut self.font_ctx, text, 1.0, true);
    push_layout_defaults(&mut builder, text_style, paragraph_style, false);

    let mut brushes = vec![None];

    for span in source.spans.iter() {
      push_span_styles(&mut builder, span, text_style, paragraph_style, &mut brushes);
    }

    let layout = builder.build(text);
    let widths = layout.calculate_content_widths();
    (widths.min, widths.max)
  }

  fn build_layout<Brush>(
    &mut self, source: &AttributedText<Brush>, text_style: &TextStyle,
    paragraph_style: &ParagraphStyle, clamp: BoxClamp, faces: &ParleyFaces,
  ) -> ParleyParagraphLayout<Brush>
  where
    Brush: Clone + From<Color> + PartialEq + 'static,
  {
    let text = source.text.as_ref();
    let mut builder = self
      .layout_ctx
      .ranged_builder(&mut self.font_ctx, text, 1.0, true);
    push_layout_defaults(&mut builder, text_style, paragraph_style, true);

    let mut brushes = vec![None];

    for span in source.spans.iter() {
      push_span_styles(&mut builder, span, text_style, paragraph_style, &mut brushes);
    }

    let mut layout = builder.build(text);
    let wrap_width = match paragraph_style.wrap {
      TextWrap::Wrap if clamp.max.width.is_finite() => Some(clamp.max.width),
      _ => None,
    };
    layout.break_all_lines(wrap_width);

    let layout = Arc::new(layout);
    let payload = Self::build_payload(layout.as_ref(), faces, &brushes);
    let logical_size = payload.bounds.size;
    let line_offsets = vec![0.; layout.len()].into_boxed_slice();

    ParleyParagraphLayout {
      layout,
      logical_size,
      payload,
      line_offsets,
      line_positions: OnceLock::new(),
    }
    .with_alignment(paragraph_style.text_align, clamp.clamp(logical_size))
  }

  fn build_payload<Brush>(
    layout: &ParleyLayout<usize>, faces: &ParleyFaces, brushes: &[Option<Brush>],
  ) -> TextDrawPayload<Brush>
  where
    Brush: Clone + PartialEq + 'static,
  {
    let mut runs = Vec::new();
    let mut decorations = Vec::new();
    let mut bounds = Rect::from_size(Size::new(layout.full_width(), layout.height()));

    for line in layout.lines() {
      for item in line.items() {
        let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
          continue;
        };
        let run = glyph_run.run();
        let font = run.font();
        let font_size = run.font_size();
        let (face_id, metrics) = Self::register_face(font, faces);
        let baseline_shift = baseline_shift(Some(metrics), font_size);
        let scale = font_metrics_scale(metrics, font_size);
        let ascender = metrics.ascender * scale;
        let descender = metrics.descender * scale;

        let mut pen_x = glyph_run.offset();
        let baseline_y = glyph_run.baseline() + baseline_shift;
        let mut run_bounds: Option<Rect> = None;
        let glyphs = glyph_run
          .glyphs()
          .map(|glyph| {
            let draw = DrawGlyph {
              glyph_id: GlyphId(glyph.id as u16),
              cluster: ClusterIndex(run.text_range().start),
              baseline_origin: Point::new(pen_x + glyph.x, baseline_y + glyph.y),
              advance: Vector::new(glyph.advance, 0.),
            };
            run_bounds = Some(union_optional_rect(
              run_bounds,
              glyph_metrics_rect(draw.baseline_origin, glyph.advance, ascender, descender),
            ));
            pen_x += glyph.advance;
            draw
          })
          .collect::<Vec<_>>()
          .into_boxed_slice();

        if let Some(run_bounds) = run_bounds {
          bounds = bounds.union(&run_bounds);
        }
        if let Some(first) = glyphs.first() {
          union_edge_glyph_rect(
            &mut bounds,
            font,
            first.glyph_id,
            font_size,
            first.baseline_origin,
          );
        }
        if glyphs.len() > 1
          && let Some(last) = glyphs.last()
        {
          union_edge_glyph_rect(&mut bounds, font, last.glyph_id, font_size, last.baseline_origin);
        }

        let underline = glyph_run.style().underline.as_ref();
        let throughline = glyph_run.style().strikethrough.as_ref();
        if underline.is_some() || throughline.is_some() {
          let default_brush = brushes
            .get(glyph_run.style().brush)
            .cloned()
            .flatten();
          let has_overline = underline.is_some_and(|decoration| decoration.offset == Some(0.));
          let has_underline =
            underline.is_some_and(|decoration| decoration.size.is_none_or(|size| size > 0.));

          if has_underline
            && let Some(rect) = push_decoration_rect(
              &mut decorations,
              TextDecoration::UNDERLINE,
              &underline
                .and_then(|decoration| brushes.get(decoration.brush).cloned().flatten())
                .or(default_brush.clone()),
              glyph_run.offset(),
              baseline_y
                - underline_offset(metrics, scale, decoration_thickness(metrics, scale, font_size)),
              glyph_run.advance(),
              decoration_thickness(metrics, scale, font_size),
            )
          {
            bounds = bounds.union(&rect);
          }
          if has_overline
            && let Some(rect) = push_decoration_rect(
              &mut decorations,
              TextDecoration::OVERLINE,
              &underline
                .and_then(|decoration| brushes.get(decoration.brush).cloned().flatten())
                .or(default_brush.clone()),
              glyph_run.offset(),
              baseline_y - overline_offset(metrics, scale),
              glyph_run.advance(),
              decoration_thickness(metrics, scale, font_size),
            )
          {
            bounds = bounds.union(&rect);
          }
          if throughline.is_some()
            && let Some(rect) = push_decoration_rect(
              &mut decorations,
              TextDecoration::THROUGHLINE,
              &throughline
                .and_then(|decoration| brushes.get(decoration.brush).cloned().flatten())
                .or(default_brush),
              glyph_run.offset(),
              baseline_y - strikeout_offset(metrics, scale),
              glyph_run.advance(),
              decoration_thickness(metrics, scale, font_size),
            )
          {
            bounds = bounds.union(&rect);
          }
        }

        runs.push(DrawGlyphRun {
          face_id,
          logical_font_size: font_size,
          brush: brushes
            .get(glyph_run.style().brush)
            .cloned()
            .flatten(),
          glyphs,
        });
      }
    }

    let shift = Vector::new((-bounds.min_x()).max(0.), (-bounds.min_y()).max(0.));
    if shift != Vector::zero() {
      bounds = bounds.translate(shift);
      decorations
        .iter_mut()
        .for_each(|decoration| decoration.rect = decoration.rect.translate(shift));
    }

    TextDrawPayload {
      bounds,
      origin_offset: shift,
      runs: runs.into_boxed_slice(),
      decorations: decorations.into_boxed_slice(),
    }
  }
}

fn push_decoration_rect<Brush: Clone>(
  decorations: &mut Vec<DrawTextDecoration<Brush>>, decoration: TextDecoration,
  brush: &Option<Brush>, x: f32, y: f32, width: f32, height: f32,
) -> Option<Rect> {
  if width <= 0. || height <= 0. {
    return None;
  }

  let rect = Rect::new(Point::new(x, y), Size::new(width, height));
  decorations.push(DrawTextDecoration { decoration, brush: brush.clone(), rect });
  Some(rect)
}

fn decoration_thickness(metrics: FontFaceMetrics, scale: f32, font_size: f32) -> f32 {
  if metrics.stroke_size > 0. { metrics.stroke_size * scale } else { (font_size / 20.).max(0.75) }
}

fn underline_offset(metrics: FontFaceMetrics, scale: f32, thickness: f32) -> f32 {
  if metrics.underline_offset != 0. {
    metrics.underline_offset * scale
  } else {
    -(metrics.descender * scale * 0.5).max(thickness)
  }
}

fn strikeout_offset(metrics: FontFaceMetrics, scale: f32) -> f32 {
  if metrics.strikeout_offset != 0. {
    metrics.strikeout_offset * scale
  } else {
    metrics.x_height.unwrap_or(metrics.ascender * 0.5) * scale * 0.5
  }
}

fn overline_offset(metrics: FontFaceMetrics, scale: f32) -> f32 {
  metrics.cap_height.unwrap_or(metrics.ascender) * scale
}

fn push_layout_defaults(
  builder: &mut parley::RangedBuilder<'_, usize>, text_style: &TextStyle,
  paragraph_style: &ParagraphStyle, include_wrap: bool,
) {
  builder.push_default(StyleProperty::Brush(0));
  builder.push_default(StyleProperty::FontFamily(font_family_for_face(&text_style.font_face)));
  builder.push_default(StyleProperty::FontSize(text_style.font_size));
  builder.push_default(StyleProperty::FontWeight(ParleyFontWeight::new(
    text_style.font_face.weight.value(),
  )));
  builder.push_default(StyleProperty::FontWidth(font_width(text_style.font_face.stretch)));
  builder.push_default(StyleProperty::FontStyle(font_style(text_style.font_face.style)));
  builder.push_default(StyleProperty::LetterSpacing(text_style.letter_space));
  if include_wrap {
    builder.push_default(StyleProperty::TextWrapMode(text_wrap_mode(paragraph_style.wrap)));
    builder.push_default(StyleProperty::OverflowWrap(match paragraph_style.wrap {
      TextWrap::NoWrap => ParleyOverflowWrap::Normal,
      TextWrap::Wrap => ParleyOverflowWrap::BreakWord,
    }));
  }
  builder.push_default(StyleProperty::LineHeight(parley_line_height(text_style.line_height)));
}

impl<Brush> Paragraph<Brush> for ParleyParagraph<Brush>
where
  Brush: Clone + From<Color> + PartialEq + 'static,
{
  fn source_len(&self) -> TextByteIndex { TextByteIndex(self.source.text.len()) }

  fn min_intrinsic_width(&self, text_style: &TextStyle, paragraph_style: &ParagraphStyle) -> f32 {
    self
      .engine
      .borrow_mut()
      .intrinsic_widths(&self.source, text_style, paragraph_style)
      .0
  }

  fn max_intrinsic_width(&self, text_style: &TextStyle, paragraph_style: &ParagraphStyle) -> f32 {
    self
      .engine
      .borrow_mut()
      .intrinsic_widths(&self.source, text_style, paragraph_style)
      .1
  }

  fn layout(
    &self, text_style: &TextStyle, paragraph_style: &ParagraphStyle, clamp: BoxClamp,
  ) -> ParagraphLayoutRef<Brush> {
    Arc::new(Box::new(self.engine.borrow_mut().build_layout(
      &self.source,
      text_style,
      paragraph_style,
      clamp,
      &self.faces,
    )))
  }
}

impl<Brush> ParagraphLayout<Brush> for ParleyParagraphLayout<Brush>
where
  Brush: Clone + PartialEq + 'static,
{
  fn size(&self) -> Size { self.logical_size }

  fn aligned(&self, text_align: TextAlign, size: Size) -> ParagraphLayoutRef<Brush> {
    Arc::new(Box::new(
      self
        .clone_for_alignment()
        .with_alignment(text_align, size),
    ))
  }

  fn draw_payload(&self) -> &TextDrawPayload<Brush> { &self.payload }

  fn hit_test_point(&self, point: Point) -> TextHitResult {
    let line = self.line_index_for_y(point.y);
    let layout_point = Point::new(
      point.x - self.payload.origin_offset.x - self.line_x_offset(line),
      point.y - self.payload.origin_offset.y,
    );
    let caret =
      self.cursor_to_caret(ParleyCursor::from_point(&self.layout, layout_point.x, layout_point.y));
    TextHitResult { caret, is_inside: self.payload.bounds.contains(point) }
  }

  fn caret_rect(&self, caret: Caret) -> Rect {
    let cursor = self.caret_to_cursor(caret);
    let caret = self.cursor_to_caret(cursor);
    if let Some(VisualPosition { line, slot }) = caret.visual {
      let (line, slot) = self.clamp_visual_position(line.0, slot);
      self.visual_line(line).caret_rect(slot)
    } else {
      let rect = rect_from_box(cursor.geometry(&self.layout, CARET_WIDTH))
        .translate(self.payload.origin_offset);
      rect.translate(Vector::new(self.line_x_offset(self.line_index_for_y(rect.center().y)), 0.))
    }
  }

  fn selection_rects(&self, selection: TextRange) -> Box<[Rect]> {
    let start =
      ParleyCursor::from_byte_index(&self.layout, selection.start.0, ParleyAffinity::Downstream);
    let end =
      ParleyCursor::from_byte_index(&self.layout, selection.end.0, ParleyAffinity::Upstream);
    ParleySelection::new(start, end)
      .geometry(&self.layout)
      .into_iter()
      .map(|(rect, _)| {
        let rect = rect_from_box(rect).translate(self.payload.origin_offset);
        rect.translate(Vector::new(self.line_x_offset(self.line_index_for_y(rect.center().y)), 0.))
      })
      .collect::<Vec<_>>()
      .into_boxed_slice()
  }

  fn move_caret(&self, caret: Caret, motion: CaretMotion) -> Caret {
    let (line, slot) = self.caret_visual(caret);
    match motion {
      CaretMotion::Prev => {
        if slot > 0 {
          self.visual_caret(line, slot - 1)
        } else if line > 0 {
          self.visual_caret(
            line - 1,
            self
              .visual_line(line - 1)
              .slots
              .len()
              .saturating_sub(1),
          )
        } else {
          self.visual_caret(0, 0)
        }
      }
      CaretMotion::Next => {
        if slot + 1 < self.visual_line(line).slots.len() {
          self.visual_caret(line, slot + 1)
        } else if line + 1 < self.line_positions().len() {
          self.visual_caret(line + 1, 0)
        } else {
          self.visual_caret(
            line,
            self
              .visual_line(line)
              .slots
              .len()
              .saturating_sub(1),
          )
        }
      }
      CaretMotion::Up => {
        self.vertical_move_caret(line.saturating_sub(1), self.visual_line(line).slots[slot].x)
      }
      CaretMotion::Down => {
        let next_line = (line + 1).min(self.line_positions().len().saturating_sub(1));
        self.vertical_move_caret(next_line, self.visual_line(line).slots[slot].x)
      }
      CaretMotion::WordPrev => {
        let cursor = self.caret_to_cursor(caret);
        let selection = ParleySelection::new(cursor, cursor);
        self.cursor_to_caret(
          selection
            .previous_visual_word(&self.layout, false)
            .focus(),
        )
      }
      CaretMotion::WordNext => {
        let cursor = self.caret_to_cursor(caret);
        let selection = ParleySelection::new(cursor, cursor);
        self.cursor_to_caret(
          selection
            .next_visual_word(&self.layout, false)
            .focus(),
        )
      }
    }
  }

  fn line_start_caret(&self, caret: Caret) -> Caret {
    let (line, _) = self.caret_visual(caret);
    self.visual_caret(line, 0)
  }

  fn line_end_caret(&self, caret: Caret) -> Caret {
    let (line, _) = self.caret_visual(caret);
    self.visual_caret(
      line,
      self
        .visual_line(line)
        .slots
        .len()
        .saturating_sub(1),
    )
  }
}

impl<Brush> ParleyParagraphLayout<Brush>
where
  Brush: Clone + PartialEq + 'static,
{
  fn clone_for_alignment(&self) -> Self {
    Self {
      layout: self.layout.clone(),
      logical_size: self.logical_size,
      payload: self.payload.clone(),
      line_offsets: self.line_offsets.clone(),
      line_positions: OnceLock::new(),
    }
  }

  fn with_alignment(self, text_align: TextAlign, size: Size) -> Self {
    let line_offsets = line_alignment_offsets(self.layout.as_ref(), size.width, text_align);
    if size == self.logical_size
      && line_offsets
        .iter()
        .all(|offset| offset.abs() <= f32::EPSILON)
    {
      return self;
    }

    let payload = shift_payload_by_line_offsets(&self.payload, self.layout.as_ref(), &line_offsets);
    Self {
      layout: self.layout,
      logical_size: size,
      payload,
      line_offsets,
      line_positions: OnceLock::new(),
    }
  }

  fn line_index_for_y(&self, y: f32) -> usize {
    let lines = self.line_positions();
    if lines.is_empty() {
      return 0;
    }
    if y < lines[0].top {
      return 0;
    }

    for (idx, line) in lines.iter().enumerate() {
      if y < line.top {
        return idx.saturating_sub(1);
      }
      if y < line.top + line.height {
        return idx;
      }
    }

    lines.len().saturating_sub(1)
  }

  fn line_x_offset(&self, line: usize) -> f32 {
    self
      .line_offsets
      .get(line)
      .copied()
      .unwrap_or_default()
  }

  fn vertical_move_caret(&self, line: usize, x: f32) -> Caret {
    let line = line.min(self.line_positions().len().saturating_sub(1));
    let slot = self.nearest_visual_slot(line, x);
    self.visual_caret(line, slot)
  }

  fn nearest_visual_slot(&self, line: usize, x: f32) -> usize {
    self
      .visual_line(line)
      .slots
      .iter()
      .enumerate()
      .min_by(|(_, a), (_, b)| {
        (a.x - x)
          .abs()
          .total_cmp(&(b.x - x).abs())
          .then_with(|| a.x.total_cmp(&b.x))
      })
      .map(|(slot, _)| slot)
      .unwrap_or(0)
  }

  fn caret_visual(&self, caret: Caret) -> (usize, usize) {
    caret
      .visual
      .or_else(|| self.find_visual(CursorKey::from_cursor(self.caret_to_cursor(caret))))
      .map(|position| (position.line.0, position.slot))
      .unwrap_or((0, 0))
  }

  fn visual_caret(&self, line: usize, slot: usize) -> Caret {
    let (line, slot) = self.clamp_visual_position(line, slot);
    let key = self.visual_line(line).slots[slot].key;
    Caret {
      byte: TextByteIndex(key.index),
      affinity: match key.affinity {
        ParleyAffinity::Upstream => CaretAffinity::Upstream,
        ParleyAffinity::Downstream => CaretAffinity::Downstream,
      },
      visual: Some(VisualPosition { line: LineIndex(line), slot }),
    }
  }

  fn caret_to_cursor(&self, caret: Caret) -> ParleyCursor {
    ParleyCursor::from_byte_index(&self.layout, caret.byte.0, parley_affinity(caret.affinity))
  }

  fn find_visual(&self, key: CursorKey) -> Option<VisualPosition> {
    self
      .line_positions()
      .iter()
      .enumerate()
      .find_map(|(line, positions)| {
        positions
          .slots
          .iter()
          .position(|position| position.key == key)
          .map(|slot| VisualPosition { line: LineIndex(line), slot })
      })
  }

  fn cursor_to_caret(&self, cursor: ParleyCursor) -> Caret {
    let key = CursorKey::from_cursor(cursor);
    let visual = self.find_visual(key);

    Caret {
      byte: TextByteIndex(cursor.index()),
      affinity: match cursor.affinity() {
        ParleyAffinity::Upstream => CaretAffinity::Upstream,
        ParleyAffinity::Downstream => CaretAffinity::Downstream,
      },
      visual,
    }
  }

  fn line_positions(&self) -> &[VisualLine] {
    self.line_positions.get_or_init(|| {
      build_line_positions(&self.layout, self.payload.origin_offset, &self.line_offsets)
    })
  }

  fn visual_line(&self, line: usize) -> &VisualLine {
    let line_positions = self.line_positions();
    &line_positions[line.min(line_positions.len().saturating_sub(1))]
  }

  fn clamp_visual_position(&self, line: usize, slot: usize) -> (usize, usize) {
    let line = line.min(self.line_positions().len().saturating_sub(1));
    let slot = slot.min(
      self
        .visual_line(line)
        .slots
        .len()
        .saturating_sub(1),
    );
    (line, slot)
  }
}

fn push_span_styles<Brush>(
  builder: &mut parley::RangedBuilder<'_, usize>, span: &TextSpan<Brush>, text_style: &TextStyle,
  paragraph_style: &ParagraphStyle, brushes: &mut Vec<Option<Brush>>,
) where
  Brush: Clone + From<Color>,
{
  let range = span.range.start.0..span.range.end.0;
  if let Some(font) = span.style.font.as_ref() {
    builder.push(StyleProperty::FontFamily(font_family_for_face(&font.face)), range.clone());
    builder.push(
      StyleProperty::FontWeight(ParleyFontWeight::new(font.face.weight.value())),
      range.clone(),
    );
    builder.push(StyleProperty::FontWidth(font_width(font.face.stretch)), range.clone());
    builder.push(StyleProperty::FontStyle(font_style(font.face.style)), range.clone());
  }
  if let Some(font_size) = span.style.font_size {
    builder.push(StyleProperty::FontSize(font_size), range.clone());
  }
  if let Some(letter_spacing) = span.style.letter_spacing {
    builder.push(StyleProperty::LetterSpacing(letter_spacing), range.clone());
  }
  let brush_idx = if let Some(brush) = span.style.brush.as_ref() {
    brushes.push(Some(brush.clone()));
    brushes.len() - 1
  } else {
    0
  };
  builder.push(StyleProperty::Brush(brush_idx), range.clone());
  if let Some(decoration) = span.style.decoration.as_ref() {
    let decoration_brush_idx = decoration.decoration_color.map(|color| {
      brushes.push(Some(Brush::from(color)));
      brushes.len() - 1
    });
    let has_underline = decoration
      .decoration
      .contains(TextDecoration::UNDERLINE);
    let has_overline = decoration
      .decoration
      .contains(TextDecoration::OVERLINE);
    let has_throughline = decoration
      .decoration
      .contains(TextDecoration::THROUGHLINE);
    if has_underline || has_overline {
      builder.push(StyleProperty::Underline(true), range.clone());
      if has_overline {
        builder.push(StyleProperty::UnderlineOffset(Some(0.)), range.clone());
      }
      if !has_underline && has_overline {
        builder.push(StyleProperty::UnderlineSize(Some(0.)), range.clone());
      }
      if let Some(brush_idx) = decoration_brush_idx {
        builder.push(StyleProperty::UnderlineBrush(Some(brush_idx)), range.clone());
      }
    }
    if has_throughline {
      builder.push(StyleProperty::Strikethrough(true), range.clone());
      if let Some(brush_idx) = decoration_brush_idx {
        builder.push(StyleProperty::StrikethroughBrush(Some(brush_idx)), range.clone());
      }
    }
  }
  builder.push(StyleProperty::TextWrapMode(text_wrap_mode(paragraph_style.wrap)), range.clone());
  let line_height = span
    .style
    .line_height
    .unwrap_or(text_style.line_height);
  builder.push(StyleProperty::LineHeight(parley_line_height(line_height)), range.clone());
}

fn parley_line_height(line_height: LineHeight) -> ParleyLineHeight {
  match line_height {
    LineHeight::Scale(value) => ParleyLineHeight::FontSizeRelative(value),
    LineHeight::Px(value) => ParleyLineHeight::Absolute(value),
  }
}

fn text_wrap_mode(wrap: TextWrap) -> TextWrapMode {
  match wrap {
    TextWrap::NoWrap => TextWrapMode::NoWrap,
    TextWrap::Wrap => TextWrapMode::Wrap,
  }
}

const CARET_WIDTH: f32 = 1.0;

fn build_line_positions(
  layout: &ParleyLayout<usize>, origin_offset: Vector, line_offsets: &[f32],
) -> Box<[VisualLine]> {
  let mut lines = Vec::new();

  for line_index in 0..layout.len() {
    let Some(line) = layout.get(line_index) else {
      continue;
    };
    let metrics = line.metrics();
    let break_reason = line.break_reason();
    let start =
      ParleyCursor::from_byte_index(layout, line.text_range().start, ParleyAffinity::Downstream);
    let line_shift = line_offsets
      .get(line_index)
      .copied()
      .unwrap_or_default();
    let line_start_x = metrics.offset + origin_offset.x + line_shift;
    let mut positions =
      vec![visual_slot(layout, start, metrics, origin_offset, line_shift, Some(line_start_x))];
    if line.is_empty() {
      lines.push(visual_line(metrics, origin_offset, positions.into_boxed_slice()));
      continue;
    }
    let mut current = ParleySelection::new(start, start);
    let mut guard = 0;
    if break_reason == BreakReason::Explicit {
      while guard < line.text_range().len() + 8 {
        let next = current.next_visual(layout, false);
        let slot = visual_slot(layout, next.focus(), metrics, origin_offset, line_shift, None);
        let raw_rect = rect_from_box(next.focus().geometry(layout, 1.0));
        let center_y = raw_rect.center().y;
        let is_same_line = center_y >= metrics.min_coord && center_y <= metrics.max_coord;
        if !is_same_line {
          break;
        }
        if positions
          .last()
          .is_some_and(|last| last.key == slot.key && last.x == slot.x)
        {
          break;
        }
        if positions
          .last()
          .is_some_and(|last| slot.x < last.x)
        {
          break;
        }
        positions.push(slot);
        current = next;
        guard += 1;
      }
    } else {
      let end = ParleySelection::new(start, start)
        .line_end(layout, false)
        .focus();
      let end_key = CursorKey::from_cursor(end);
      let line_end_x = metrics.offset + metrics.advance + origin_offset.x + line_shift;
      while CursorKey::from_cursor(current.focus()) != end_key
        && guard < line.text_range().len() + 8
      {
        let next = current.next_visual(layout, false);
        let slot = visual_slot(layout, next.focus(), metrics, origin_offset, line_shift, None);
        if positions
          .last()
          .is_some_and(|last| last.key == slot.key && last.x == slot.x)
        {
          break;
        }
        positions.push(slot);
        current = next;
        guard += 1;
      }
      let end_slot = visual_slot(layout, end, metrics, origin_offset, line_shift, Some(line_end_x));
      if positions.last().map(|slot| slot.key) != Some(end_key) {
        positions.push(end_slot);
      }
    }
    lines.push(visual_line(metrics, origin_offset, positions.into_boxed_slice()));
  }

  if lines.is_empty() {
    let cursor = ParleyCursor::from_byte_index(layout, 0, ParleyAffinity::Downstream);
    let rect = rect_from_box(cursor.geometry(layout, 1.0));
    let height = rect.height();
    let metrics = parley::layout::LineMetrics {
      ascent: 0.,
      descent: 0.,
      leading: 0.,
      line_height: height,
      baseline: rect.max_y(),
      offset: 0.,
      advance: 0.,
      trailing_whitespace: 0.,
      min_coord: rect.min_y(),
      max_coord: rect.max_y(),
    };
    lines.push(visual_line(
      &metrics,
      origin_offset,
      vec![visual_slot(layout, cursor, &metrics, origin_offset, 0., Some(origin_offset.x))]
        .into_boxed_slice(),
    ));
  }

  lines.into_boxed_slice()
}

fn visual_line(
  metrics: &parley::layout::LineMetrics, origin_offset: Vector, slots: Box<[VisualSlot]>,
) -> VisualLine {
  VisualLine {
    top: metrics.min_coord + origin_offset.y,
    height: metrics.max_coord - metrics.min_coord,
    slots,
  }
}

fn visual_slot(
  layout: &ParleyLayout<usize>, cursor: ParleyCursor, _metrics: &parley::layout::LineMetrics,
  origin_offset: Vector, line_shift: f32, x_override: Option<f32>,
) -> VisualSlot {
  let layout_rect = rect_from_box(cursor.geometry(layout, CARET_WIDTH));
  let x = x_override.unwrap_or(layout_rect.min_x() + origin_offset.x + line_shift);
  VisualSlot { key: CursorKey::from_cursor(cursor), x }
}

fn line_alignment_offsets(
  layout: &ParleyLayout<usize>, width: f32, text_align: TextAlign,
) -> Box<[f32]> {
  if !width.is_finite() {
    return vec![0.; layout.len()].into_boxed_slice();
  }

  let is_rtl = layout.is_rtl();
  layout
    .lines()
    .map(|line| {
      let metrics = line.metrics();
      let content_width = (metrics.advance - metrics.trailing_whitespace).max(0.);
      let free_space = (width - content_width).max(0.);
      let base_offset = if is_rtl { -metrics.trailing_whitespace } else { 0. };
      match (text_align, is_rtl) {
        (TextAlign::Start, false) => base_offset,
        (TextAlign::Start, true) => base_offset + free_space,
        (TextAlign::Center, _) => base_offset + free_space * 0.5,
        (TextAlign::End, false) => base_offset + free_space,
        (TextAlign::End, true) => base_offset,
      }
    })
    .collect::<Vec<_>>()
    .into_boxed_slice()
}

fn shift_payload_by_line_offsets<Brush: Clone>(
  payload: &TextDrawPayload<Brush>, layout: &ParleyLayout<usize>, line_offsets: &[f32],
) -> TextDrawPayload<Brush> {
  let mut shifted = payload.clone();
  let mut run_idx = 0;
  let mut decoration_idx = 0;
  let mut min_offset: f32 = 0.;
  let mut max_offset: f32 = 0.;

  for (line_idx, line) in layout.lines().enumerate() {
    let line_offset = line_offsets
      .get(line_idx)
      .copied()
      .unwrap_or_default();
    min_offset = min_offset.min(line_offset);
    max_offset = max_offset.max(line_offset);

    for item in line.items() {
      let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
        continue;
      };

      if line_offset != 0.
        && let Some(run) = shifted.runs.get_mut(run_idx)
      {
        run
          .glyphs
          .iter_mut()
          .for_each(|glyph| glyph.baseline_origin.x += line_offset);
      }
      run_idx += 1;
      decoration_idx = shift_run_decorations(
        &mut shifted.decorations,
        decoration_idx,
        glyph_run.style(),
        line_offset,
      );
    }
  }

  if min_offset != 0. || max_offset != 0. {
    shifted.bounds = Rect::new(
      Point::new(shifted.bounds.min_x() + min_offset, shifted.bounds.min_y()),
      Size::new(shifted.bounds.width() + max_offset - min_offset, shifted.bounds.height()),
    );
    let normalize_x = (-shifted.bounds.min_x()).max(0.);
    if normalize_x > 0. {
      let shift = Vector::new(normalize_x, 0.);
      shifted.runs.iter_mut().for_each(|run| {
        run
          .glyphs
          .iter_mut()
          .for_each(|glyph| glyph.baseline_origin.x += normalize_x)
      });
      shifted
        .decorations
        .iter_mut()
        .for_each(|decoration| decoration.rect = decoration.rect.translate(shift));
      shifted.bounds = shifted.bounds.translate(shift);
      shifted.origin_offset += shift;
    }
  }

  shifted
}

fn shift_run_decorations<Brush>(
  decorations: &mut [DrawTextDecoration<Brush>], mut decoration_idx: usize,
  style: &parley::layout::Style<usize>, line_offset: f32,
) -> usize {
  let underline = style.underline.as_ref();
  let throughline = style.strikethrough.as_ref();
  let has_overline = underline.is_some_and(|decoration| decoration.offset == Some(0.));
  let has_underline =
    underline.is_some_and(|decoration| decoration.size.is_none_or(|size| size > 0.));

  if line_offset != 0. {
    let shift = Vector::new(line_offset, 0.);
    let mut translate_next = |idx: &mut usize| {
      if let Some(decoration) = decorations.get_mut(*idx) {
        decoration.rect = decoration.rect.translate(shift);
      }
      *idx += 1;
    };

    if has_underline {
      translate_next(&mut decoration_idx);
    }
    if has_overline {
      translate_next(&mut decoration_idx);
    }
    if throughline.is_some() {
      translate_next(&mut decoration_idx);
    }
  } else {
    decoration_idx +=
      has_underline as usize + has_overline as usize + throughline.is_some() as usize;
  }

  decoration_idx
}

impl VisualLine {
  fn caret_rect(&self, slot: usize) -> Rect {
    let slot = slot.min(self.slots.len().saturating_sub(1));
    Rect::new(Point::new(self.slots[slot].x, self.top), Size::new(CARET_WIDTH, self.height))
  }
}

fn rect_from_box(rect: parley::BoundingBox) -> Rect {
  Rect::new(
    Point::new(rect.x0 as f32, rect.y0 as f32),
    Size::new(rect.width() as f32, rect.height() as f32),
  )
}

fn parley_affinity(affinity: CaretAffinity) -> ParleyAffinity {
  match affinity {
    CaretAffinity::Upstream => ParleyAffinity::Upstream,
    CaretAffinity::Downstream => ParleyAffinity::Downstream,
  }
}

fn font_family_for_face(face: &crate::FontFace) -> ParleyFontFamily<'static> {
  let mut families = Vec::new();

  for family in face.families.iter() {
    match family {
      FontFamily::Name(name) => families.push(ParleyFontFamilyName::Named(name.to_string().into())),
      FontFamily::Serif => families.push(ParleyFontFamilyName::Generic(GenericFamily::Serif)),
      FontFamily::SansSerif => {
        families.push(ParleyFontFamilyName::Generic(GenericFamily::SansSerif))
      }
      FontFamily::Cursive => families.push(ParleyFontFamilyName::Generic(GenericFamily::Cursive)),
      FontFamily::Fantasy => families.push(ParleyFontFamilyName::Generic(GenericFamily::Fantasy)),
      FontFamily::Monospace => {
        families.push(ParleyFontFamilyName::Generic(GenericFamily::Monospace))
      }
    }
  }

  ParleyFontFamily::List(families.into())
}

fn font_style(style: FontStyle) -> ParleyFontStyle { style }

fn font_width(stretch: FontStretch) -> ParleyFontWidth { stretch }

fn font_metrics(font: &ParleyFontData) -> FontFaceMetrics {
  let font_ref = FontRef::from_index(font.data.data(), font.index as usize).unwrap();
  let metrics = font_ref.metrics(&[]);
  FontFaceMetrics {
    units_per_em: metrics.units_per_em,
    vertical_height: Some(metrics.ascent - metrics.descent),
    ascender: metrics.ascent,
    descender: -metrics.descent,
    line_gap: metrics.leading,
    x_height: Some(metrics.x_height),
    cap_height: Some(metrics.cap_height),
    underline_offset: metrics.underline_offset,
    strikeout_offset: metrics.strikeout_offset,
    stroke_size: metrics.stroke_size,
  }
}

fn font_metrics_scale(metrics: FontFaceMetrics, font_size: f32) -> f32 {
  font_size / f32::from(metrics.units_per_em.max(1))
}

fn union_optional_rect(bounds: Option<Rect>, rect: Rect) -> Rect {
  bounds.map_or(rect, |bounds| bounds.union(&rect))
}

fn glyph_metrics_rect(baseline_origin: Point, advance: f32, ascender: f32, descender: f32) -> Rect {
  Rect::new(
    Point::new(baseline_origin.x, baseline_origin.y - ascender),
    Size::new(advance.max(0.), (ascender + descender).max(0.)),
  )
}

fn baseline_shift(metrics: Option<FontFaceMetrics>, font_size: f32) -> f32 {
  let _ = (metrics, font_size);
  0.
}

fn glyph_raster_rect(
  font: &ParleyFontData, glyph_id: GlyphId, font_size: f32, baseline_origin: Point,
) -> Option<Rect> {
  let img_size = font_size.ceil().max(1.) as u16;
  let scale = font_size / img_size as f32;
  let bitmap = raster_bitmap_from_font(font, glyph_id, img_size)?;

  Some(Rect::new(
    Point::new(
      baseline_origin.x + bitmap.placement.x * scale,
      baseline_origin.y + bitmap.placement.y * scale,
    ),
    Size::new(bitmap.width as f32 * scale, bitmap.height as f32 * scale),
  ))
}

fn union_edge_glyph_rect(
  bounds: &mut Rect, font: &ParleyFontData, glyph_id: GlyphId, font_size: f32,
  baseline_origin: Point,
) {
  if let Some(rect) = glyph_raster_rect(font, glyph_id, font_size, baseline_origin) {
    *bounds = bounds.union(&rect);
  }
}

fn subpixel_mask_to_alpha(data: Vec<u8>) -> Vec<u8> {
  let mut pixels = data.chunks_exact(4);
  let alpha = pixels
    .by_ref()
    .map(|pixel| {
      let coverage = u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
      (coverage / 3) as u8
    })
    .collect();
  assert!(
    pixels.remainder().is_empty(),
    "swash::scale::image::Content::SubpixelMask should use RGBA pixels",
  );
  alpha
}

fn raster_bitmap_from_font(
  font: &ParleyFontData, glyph_id: GlyphId, img_size: u16,
) -> Option<RasterBitmap> {
  let font_ref = FontRef::from_index(font.data.data(), font.index as usize)?;
  let mut scale_ctx = swash::scale::ScaleContext::new();
  let mut scaler = scale_ctx
    .builder(font_ref)
    .size(img_size as f32)
    .hint(true)
    .build();
  let image = swash::scale::Render::new(&[
    swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
    swash::scale::Source::Bitmap(swash::scale::StrikeWith::BestFit),
    swash::scale::Source::Outline,
  ])
  .render(&mut scaler, glyph_id.0);

  image.map(|img| {
    let placement = img.placement;
    let (data, format) = match img.content {
      swash::scale::image::Content::Mask => (img.data, RasterBitmapFormat::Alpha8),
      swash::scale::image::Content::SubpixelMask => {
        (subpixel_mask_to_alpha(img.data), RasterBitmapFormat::Alpha8)
      }
      swash::scale::image::Content::Color => (img.data, RasterBitmapFormat::Rgba8),
    };
    RasterBitmap {
      data,
      width: placement.width,
      height: placement.height,
      format,
      placement: Point::new(placement.left as f32, -placement.top as f32),
    }
  })
}

pub(crate) struct ParleyFontSystem {
  engine: Rc<std::cell::RefCell<ParleyEngine>>,
  faces: ParleyFaces,
}

impl Default for ParleyFontSystem {
  fn default() -> Self {
    let mut this = ParleyFontSystem {
      engine: Rc::new(std::cell::RefCell::new(ParleyEngine::new())),
      faces: Arc::new(RwLock::new(std::collections::HashMap::new())),
    };
    let bytes = include_bytes!("Lato-Regular.ttf").to_vec();
    let _ = this.register_font_bytes(bytes);
    this
  }
}

impl ParleyFontSystem {
  pub(crate) fn paragraph<Brush>(&self, source: AttributedText<Brush>) -> Rc<dyn Paragraph<Brush>>
  where
    Brush: Clone + From<Color> + PartialEq + 'static,
  {
    build_text_paragraph(source, self.engine.clone(), self.faces.clone())
  }
}

impl FontSystem for ParleyFontSystem {
  fn register_font_bytes(&mut self, data: Vec<u8>) -> Result<(), FontLoadError> {
    self.engine.borrow_mut().register_font_bytes(data);

    Ok(())
  }

  fn register_font_file(&mut self, path: &std::path::Path) -> Result<(), FontLoadError> {
    let data = std::fs::read(path).map_err(|e| FontLoadError::new(e.to_string()))?;
    self.register_font_bytes(data)
  }

  fn face_metrics(&self, face: FontFaceId) -> Option<FontFaceMetrics> {
    self
      .faces
      .read()
      .unwrap()
      .get(&face)
      .map(|face| face.metrics)
  }

  fn raster_source(&self) -> GlyphRasterSourceRef { build_text_raster_source(self.faces.clone()) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug, Clone, PartialEq, Eq)]
  struct TestBrush(u8);

  impl From<crate::Color> for TestBrush {
    fn from(color: crate::Color) -> Self { Self(color.red) }
  }

  fn union_ink_rect(bounds: Option<Rect>, rect: Rect) -> Rect {
    bounds.map_or(rect, |bounds| bounds.union(&rect))
  }

  fn actual_ink_bounds(
    payload: &TextDrawPayload<TestBrush>, services: &dyn crate::TextServices<TestBrush>,
  ) -> Option<Rect> {
    let raster_source = services.raster_source();
    let mut bounds = None;

    for run in payload.runs.iter() {
      let img_size = run.logical_font_size.ceil().max(1.) as u16;
      let scale = run.logical_font_size / img_size as f32;
      for glyph in run.glyphs.iter() {
        let Some(bitmap) = raster_source.raster_bitmap(run.face_id, glyph.glyph_id, img_size)
        else {
          continue;
        };
        let rect = Rect::new(
          Point::new(
            glyph.baseline_origin.x + payload.origin_offset.x + bitmap.placement.x * scale,
            glyph.baseline_origin.y + payload.origin_offset.y + bitmap.placement.y * scale,
          ),
          Size::new(bitmap.width as f32 * scale, bitmap.height as f32 * scale),
        );
        bounds = Some(union_ink_rect(bounds, rect));
      }
    }

    bounds
  }

  fn dejavu_face() -> crate::FontFace {
    crate::FontFace {
      families: Box::new([crate::FontFamily::Name("DejaVu Sans".into())]),
      ..Default::default()
    }
  }

  fn register_test_font(services: &dyn crate::TextServices<TestBrush>) {
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = services.register_font_file(std::path::Path::new(&path));
  }

  #[test]
  fn subpixel_mask_is_collapsed_to_alpha_coverage() {
    let alpha = subpixel_mask_to_alpha(vec![12, 24, 60, 0, 255, 255, 255, 0]);

    assert_eq!(alpha, vec![32, 255]);
  }

  #[test]
  fn text_style_line_height_is_paragraph_default() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::styled(
      "A\nB",
      crate::SpanStyle {
        font: Some(crate::FontRequest { face: dejavu_face() }),
        font_size: Some(16.),
        letter_spacing: Some(0.),
        line_height: None,
        brush: None,
        decoration: None,
      },
    );
    let paragraph = services.paragraph(source);
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::Wrap };

    let compact = crate::TextStyle {
      font_size: 16.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::Px(16.),
      overflow: crate::TextOverflow::AutoWrap,
    };
    let spacious = crate::TextStyle { line_height: crate::LineHeight::Px(40.), ..compact.clone() };

    let compact_layout =
      paragraph.layout(&compact, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));
    let spacious_layout =
      paragraph.layout(&spacious, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));

    assert!(spacious_layout.size().height > compact_layout.size().height);
  }

  #[test]
  fn draw_payload_preserves_run_brush_overrides() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::from_parts(
      ribir_algo::CowArc::from("AB"),
      vec![
        crate::TextSpan {
          range: crate::TextRange::new(0, 1),
          style: crate::SpanStyle {
            font: Some(crate::FontRequest { face: dejavu_face() }),
            font_size: Some(16.),
            letter_spacing: Some(0.),
            line_height: None,
            brush: None,
            decoration: None,
          },
        },
        crate::TextSpan {
          range: crate::TextRange::new(1, 2),
          style: crate::SpanStyle {
            font: Some(crate::FontRequest { face: dejavu_face() }),
            font_size: Some(16.),
            letter_spacing: Some(0.),
            line_height: None,
            brush: Some(TestBrush(7)),
            decoration: None,
          },
        },
      ]
      .into_boxed_slice(),
    );

    let paragraph = services.paragraph(source);
    let text_style = crate::TextStyle {
      font_size: 16.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::Px(16.),
      overflow: crate::TextOverflow::Overflow,
    };
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::NoWrap };
    let layout =
      paragraph.layout(&text_style, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));
    let payload = layout.draw_payload();

    assert!(payload.runs.iter().any(|run| run.brush.is_none()));
    assert!(
      payload
        .runs
        .iter()
        .any(|run| run.brush == Some(TestBrush(7)))
    );
  }

  #[test]
  fn payload_bounds_cover_large_text_ink_bounds() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::styled(
      "accentgjy",
      crate::SpanStyle {
        font: Some(crate::FontRequest { face: dejavu_face() }),
        font_size: Some(64.),
        letter_spacing: Some(0.),
        line_height: None,
        brush: None,
        decoration: None,
      },
    );
    let paragraph = services.paragraph(source);
    let text_style = crate::TextStyle {
      font_size: 64.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::Px(64.),
      overflow: crate::TextOverflow::Overflow,
    };
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::NoWrap };
    let layout =
      paragraph.layout(&text_style, &paragraph_style, BoxClamp::max_size(Size::new(500., 200.)));
    let payload = layout.draw_payload();
    let actual = actual_ink_bounds(payload, services.as_ref()).expect("expected glyph ink bounds");

    assert!(payload.bounds.min_x() >= 0.);
    assert!(payload.bounds.min_y() >= 0.);
    assert!(payload.bounds.min_x() <= actual.min_x());
    assert!(payload.bounds.min_y() <= actual.min_y());
    assert!(payload.bounds.max_x() >= actual.max_x());
    assert!(payload.bounds.max_y() >= actual.max_y());
  }

  #[test]
  fn normalized_origin_keeps_interaction_geometry_aligned() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::styled(
      "accentgjy",
      crate::SpanStyle {
        font: Some(crate::FontRequest { face: dejavu_face() }),
        font_size: Some(64.),
        letter_spacing: Some(0.),
        line_height: None,
        brush: None,
        decoration: None,
      },
    );
    let paragraph = services.paragraph(source);
    let text_style = crate::TextStyle {
      font_size: 64.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::Px(64.),
      overflow: crate::TextOverflow::Overflow,
    };
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::NoWrap };
    let layout =
      paragraph.layout(&text_style, &paragraph_style, BoxClamp::max_size(Size::new(500., 200.)));

    let bounds = layout.draw_payload().bounds;
    let caret = layout.caret_rect(Caret::default());
    let selection = layout.selection_rects(crate::TextRange::new(0, 1));
    let hit = layout.hit_test_point(Point::new(caret.min_x(), caret.center().y));

    assert!(bounds.contains(caret.origin));
    assert!(
      selection
        .iter()
        .all(|rect| bounds.intersects(rect))
    );
    assert_eq!(hit.caret.byte, TextByteIndex(0));
  }

  #[test]
  fn relative_line_height_matches_equivalent_absolute_height() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::styled(
      "A\nB",
      crate::SpanStyle {
        font: Some(crate::FontRequest { face: dejavu_face() }),
        font_size: Some(20.),
        letter_spacing: Some(0.),
        line_height: None,
        brush: None,
        decoration: None,
      },
    );
    let paragraph = services.paragraph(source);
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::Wrap };

    let number = crate::TextStyle {
      font_size: 20.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::Scale(1.5),
      overflow: crate::TextOverflow::AutoWrap,
    };
    let absolute = crate::TextStyle { line_height: crate::LineHeight::Px(30.), ..number.clone() };

    let relative_layout =
      paragraph.layout(&number, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));
    let absolute_layout =
      paragraph.layout(&absolute, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));

    let relative_height = relative_layout.size().height;
    let absolute_height = absolute_layout.size().height;

    assert!((relative_height - absolute_height).abs() < 0.1);
  }

  #[test]
  fn default_relative_line_height_differs_from_same_absolute_height() {
    let services = crate::new_text_services::<TestBrush>();
    register_test_font(services.as_ref());

    let source = crate::AttributedText::styled(
      "A\nB",
      crate::SpanStyle {
        font: Some(crate::FontRequest { face: dejavu_face() }),
        font_size: Some(20.),
        letter_spacing: Some(0.),
        line_height: None,
        brush: None,
        decoration: None,
      },
    );
    let paragraph = services.paragraph(source);
    let paragraph_style =
      crate::ParagraphStyle { text_align: crate::TextAlign::Start, wrap: crate::TextWrap::Wrap };

    let relative = crate::TextStyle {
      font_size: 20.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::LineHeight::default(),
      overflow: crate::TextOverflow::AutoWrap,
    };
    let absolute = crate::TextStyle { line_height: crate::LineHeight::Px(20.), ..relative.clone() };

    let relative_layout =
      paragraph.layout(&relative, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));
    let absolute_layout =
      paragraph.layout(&absolute, &paragraph_style, BoxClamp::max_size(Size::new(200., 200.)));

    assert!((relative_layout.size().height - absolute_layout.size().height).abs() > 0.1);
  }
}
