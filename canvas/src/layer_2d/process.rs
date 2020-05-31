use super::{
  Color, Command, CommandInfo, FillStyle, HorizontalAlign, RenderAttr, RenderCommand,
  Rendering2DLayer, Text, TextLayout, Transform, Vertex, VerticalAlign,
};
use crate::{
  canvas::{surface::Surface, Canvas},
  text::Section,
  LogicUnit, Point, Rect, Size,
};
pub use lyon::{
  path::{builder::PathBuilder, traits::PathIterator, Path, Winding},
  tessellation::*,
};
use smallvec::{smallvec, SmallVec};

const TOLERANCE: f32 = 0.5;

pub(crate) struct ProcessLayer2d<'a, S: Surface> {
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
  geometry: VertexBuffers<Vertex, u32>,
  attrs: Vec<RenderAttr>,
  unprocessed_text_attrs: Vec<UnprocessedTextAttr<'a>>,
  texture_updated: bool,
  queued_text: bool,
  canvas: &'a mut Canvas<S>,
}

struct UnprocessedTextAttr<'a> {
  sec: Section<'a>,
  sec_rect: Option<euclid::Box2D<f32, LogicUnit>>,
  transform: Transform,
  align_bounds: Rect,
  attrs: SmallVec<[FillStyle; 1]>,
}

impl<'a, S: Surface> ProcessLayer2d<'a, S> {
  pub(crate) fn new(canvas: &'a mut Canvas<S>) -> Self {
    Self {
      stroke_tess: <_>::default(),
      fill_tess: FillTessellator::new(),
      geometry: VertexBuffers::new(),
      attrs: vec![],
      unprocessed_text_attrs: vec![],
      texture_updated: false,
      queued_text: false,
      canvas,
    }
  }

  pub(crate) fn process_layer(mut self, layer: Rendering2DLayer<'a>) -> Option<RenderCommand> {
    layer
      .commands
      .into_iter()
      .for_each(|Command { transform, info }| {
        match info {
          CommandInfo::Path {
            path,
            style,
            stroke_width,
          } => {
            self.tessellate_path(path, style, stroke_width, transform);
          }
          CommandInfo::SimpleText {
            text,
            style,
            max_width,
          } => {
            self.queue_simple_text(text, style, max_width, transform);
          }
          CommandInfo::ComplexTexts {
            texts,
            bounds,
            layout,
          } => {
            self.queue_complex_texts(texts, transform, bounds, layout);
          }
          CommandInfo::ComplexTextsByStyle {
            style,
            texts,
            bounds,
            layout,
          } => {
            self.queue_complex_texts_by_style(texts, transform, style, bounds, layout);
          }
        };
      });

    self.process_queued_text();

    let cmd = RenderCommand {
      geometry: self.geometry,
      attrs: self.attrs,
    };
    if self.texture_updated {
      self.canvas.upload_render_command(&cmd);
      None
    } else {
      Some(cmd)
    }
  }

  fn process_queued_text(&mut self) {
    if !self.queued_text {
      return;
    }

    let Self {
      canvas, geometry, ..
    } = self;
    let (quad_vertices, texture_updated) = canvas.process_queued();
    self.texture_updated = texture_updated;

    let count = glyphs_geometry_count(quad_vertices.len());

    geometry.vertices.reserve(count.vertices as usize);
    geometry.indices.reserve(count.indices as usize);

    fn rect_corners(rect: &Rect) -> [Point; 4] {
      [
        rect.min(),
        Point::new(rect.max_x(), rect.min_y()),
        Point::new(rect.min_x(), rect.max_y()),
        rect.max(),
      ]
    }
    quad_vertices.iter().for_each(|v| {
      let VertexBuffers { vertices, indices } = geometry;
      let offset = vertices.len() as u32;
      let tl = offset;
      let tr = 1 + offset;
      let bl = 2 + offset;
      let br = 3 + offset;
      indices.push(tl);
      indices.push(tr);
      indices.push(bl);
      indices.push(bl);
      indices.push(tr);
      indices.push(br);

      let px_coords = rect_corners(&v.pixel_coords);
      let tex_coords = rect_corners(&v.tex_coords);
      vertices.push(Vertex {
        pixel_coords: px_coords[0],
        texture_coords: tex_coords[0],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[1],
        texture_coords: tex_coords[1],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[2],
        texture_coords: tex_coords[2],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[3],
        texture_coords: tex_coords[3],
      });
    });
    self.queued_text = false;
    self.process_text_attrs();
  }

  fn tessellate_path(
    &mut self,
    path: Path,
    style: FillStyle,
    stroke_width: Option<f32>,
    transform: Transform,
  ) {
    // ensure all queued text has be processed.
    self.process_queued_text();

    let count = if let Some(line_width) = stroke_width {
      self
        .stroke_tess
        .tessellate_path(
          &path,
          &StrokeOptions::tolerance(TOLERANCE).with_line_width(line_width),
          &mut BuffersBuilder::new(&mut self.geometry, Vertex::from_stroke_vertex),
        )
        .unwrap()
    } else {
      self
        .fill_tess
        .tessellate_path(
          &path,
          &FillOptions::tolerance(TOLERANCE),
          &mut BuffersBuilder::new(&mut self.geometry, Vertex::from_fill_vertex),
        )
        .unwrap()
    };
    let bounds = path_bounds_to_align_texture(&style, &path);
    self.add_attr_and_try_merge(count, transform, style, bounds);
  }

  fn queue_simple_text(
    &mut self,
    text: Text<'a>,
    style: FillStyle,
    max_width: Option<f32>,
    transform: Transform,
  ) {
    let mut sec = Section::new().add_text(text.to_glyph_text(self.canvas));
    if let Some(max_width) = max_width {
      sec.bounds = (max_width, f32::INFINITY).into()
    }
    let align_bounds = section_bounds_to_align_texture(self.canvas, &style, &sec);
    if !align_bounds.is_empty_or_negative() {
      self.queue_section(&sec);
      self.unprocessed_text_attrs.push(UnprocessedTextAttr {
        sec,
        transform,
        align_bounds,
        sec_rect: None,
        attrs: smallvec![style],
      });
    }
  }
  fn queue_complex_texts(
    &mut self,
    texts: Vec<(Text<'a>, Color)>,
    transform: Transform,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let mut attrs = SmallVec::with_capacity(texts.len());
    let texts = texts
      .into_iter()
      .map(|(t, color)| {
        attrs.push(FillStyle::Color(color));
        t.to_glyph_text(self.canvas)
      })
      .collect();
    let mut sec = Section::new().with_text(texts);
    sec = section_with_layout_bounds(sec, bounds, layout);

    self.queue_section(&sec);

    self.unprocessed_text_attrs.push(UnprocessedTextAttr {
      sec,
      transform,
      sec_rect: bounds.map(|r| r.to_box2d()),
      align_bounds: COLOR_BOUNDS_TO_ALIGN_TEXTURE,
      attrs,
    })
  }

  fn queue_complex_texts_by_style(
    &mut self,
    texts: Vec<Text<'a>>,
    transform: Transform,
    style: FillStyle,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let texts = texts
      .into_iter()
      .map(|t| t.to_glyph_text(self.canvas))
      .collect();
    let mut sec = Section::new().with_text(texts);
    let align_bounds = section_bounds_to_align_texture(self.canvas, &style, &sec);
    if !align_bounds.is_empty_or_negative() {
      sec = section_with_layout_bounds(sec, bounds, layout);
      self.queue_section(&sec);
      self.unprocessed_text_attrs.push(UnprocessedTextAttr {
        sec,
        transform,
        sec_rect: bounds.map(|r| r.to_box2d()),
        align_bounds,
        attrs: smallvec![style],
      });
    }
  }

  fn process_text_attrs(&mut self) {
    let Self {
      canvas,
      attrs: render_attrs,
      unprocessed_text_attrs,
      ..
    } = self;
    let ptr = &canvas.glyph_brush as *const crate::text::TextBrush;
    unprocessed_text_attrs.drain(..).for_each(
      |UnprocessedTextAttr {
         sec,
         attrs,
         align_bounds,
         sec_rect,
         transform,
       }| {
        let single_attr = attrs.len() == 1;
        let mut glyph_counts = vec![0; attrs.len()];
        canvas
          .glyph_brush
          .glyphs(&sec)
          .filter(|g| {
            // unsafe introduce:
            // `glyph_brush.glyphs` need mut reference of glyph_brush, but its return
            // iterator is not, so it's safe here to reference glyph_brush.
            // use unsafe to avoid create a vector to store glyphs.
            let draw_rect = unsafe { (&*ptr).draw_rect_for_cache(g) };
            draw_rect
              .map(|rect| {
                sec_rect
                  .map(|sec_rect| {
                    let min = Point::new(rect.min.x, rect.min.y);
                    let max = Point::new(rect.max.x, rect.max.y);
                    let glyph_rect = euclid::Box2D::new(min, max);
                    sec_rect.intersects(&glyph_rect)
                  })
                  .unwrap_or(true)
              })
              .unwrap_or(false)
          })
          .for_each(|g| {
            let attr_idx = if single_attr { 0 } else { g.section_index };
            glyph_counts[attr_idx] += 1;
          });

        let text_attrs =
          attrs
            .into_iter()
            .zip(glyph_counts.into_iter())
            .map(|(style, draw_count)| RenderAttr {
              transform,
              count: glyphs_geometry_count(draw_count),
              style,
              bounding_to_align_texture: align_bounds,
            });

        render_attrs.extend(text_attrs);
      },
    );

    unprocessed_text_attrs.clear();
  }

  fn queue_section(&mut self, sec: &Section) {
    self.canvas.queue(&sec);
    self.queued_text = true;
  }

  fn add_attr_and_try_merge(
    &mut self,
    count: Count,
    transform: Transform,
    style: FillStyle,
    bounds: Rect,
  ) {
    if let Some(last) = self.attrs.last_mut() {
      if last.bounding_to_align_texture == bounds
        && last.style == style
        && last.transform == transform
      {
        last.count.vertices += count.vertices;
        last.count.indices += count.indices;
        return;
      }
    }

    self.attrs.push(RenderAttr {
      transform,
      bounding_to_align_texture: bounds,
      count,
      style: style.clone(),
    });
  }
}

#[inline]
fn glyphs_geometry_count(glyph_count: usize) -> Count {
  let glyph_count = glyph_count as u32;
  Count {
    vertices: glyph_count * 4,
    indices: glyph_count * 6,
  }
}

fn section_with_layout_bounds(
  mut sec: Section,
  bounds: Option<Rect>,
  layout: Option<TextLayout>,
) -> Section {
  if let Some(layout) = layout {
    sec = sec.with_layout(layout);
  }
  if let Some(bounds) = bounds {
    sec = section_with_bounds(sec, bounds);
  }
  sec
}

fn section_with_bounds(mut sec: Section, bounds: Rect) -> Section {
  sec = sec.with_bounds(bounds.size);

  let (h_align, v_align) = match &sec.layout {
    glyph_brush::Layout::SingleLine {
      h_align, v_align, ..
    } => (h_align, v_align),
    glyph_brush::Layout::Wrap {
      h_align, v_align, ..
    } => (h_align, v_align),
  };

  let mut pos = bounds.min();
  match h_align {
    HorizontalAlign::Left => {}
    HorizontalAlign::Center => pos.x = bounds.center().x,
    HorizontalAlign::Right => pos.x = bounds.max_x(),
  }
  match v_align {
    VerticalAlign::Top => {}
    VerticalAlign::Center => pos.y = bounds.center().y,
    VerticalAlign::Bottom => pos.y = bounds.max_y(),
  }
  sec.with_screen_position(pos)
}

// Pure color just one pixel in texture, and always use repeat pattern, so
// zero min is ok, no matter what really bounding it is.
const COLOR_BOUNDS_TO_ALIGN_TEXTURE: Rect = Rect::new(Point::new(0., 0.), Size::new(1., 1.));

fn path_bounds_to_align_texture(style: &FillStyle, path: &Path) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    let rect = lyon::algorithms::aabb::bounding_rect(path.iter());
    Rect::from_untyped(&rect)
  }
}

fn section_bounds_to_align_texture<S: Surface>(
  canvas: &mut Canvas<S>,
  style: &FillStyle,
  sec: &Section,
) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    canvas.glyph_brush.glyph_bounds(sec).unwrap_or(Rect::zero())
  }
}

#[cfg(test)]
mod tests {
  use super::super::const_color;
  use super::*;

  #[test]
  fn bounding_align() {
    let mut path = Path::builder();
    path.add_rectangle(&lyon::geom::rect(100., 100., 50., 50.), Winding::Positive);
    let path = path.build();

    let rect = path_bounds_to_align_texture(&FillStyle::Color(const_color::BLACK.into()), &path);
    assert_eq!(rect, Rect::from_size(Size::new(1., 1.)));

    let rect = path_bounds_to_align_texture(&FillStyle::Image, &path);
    assert_eq!(rect.min(), Point::new(100., 100.));
    assert_eq!(rect.size, Size::new(50., 50.));
  }

  #[test]
  fn section_bounds_layout() {
    let section = Section::new();
    let rect = euclid::rect(10., 20., 40., 30.);
    let layout = TextLayout::default();

    let l = layout.clone();
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    assert_eq!(s.screen_position, rect.min().into());
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.center().x, rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Right;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.max_x(), rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.v_align = VerticalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.center().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.v_align = VerticalAlign::Bottom;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.max_y());
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());
  }
}
