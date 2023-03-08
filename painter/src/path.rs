use crate::{
  path_builder::{stroke_path, PathBuilder},
  Point, Rect, Transform,
};
use lyon_algorithms::path::Path as LyonPath;
use serde::{Deserialize, Serialize};

/// Path widget describe a shape, build the shape from [`Builder`]!
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Path(pub(crate) LyonPath);

/// Describe how to paint path, fill or stroke.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum PathPaintStyle {
  /// Fill the path.
  #[default]
  Fill,
  /// Stroke path with line width.
  Stroke(StrokeOptions),
}

/// Stroke properties.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct StrokeOptions {
  /// A stroke thickness.
  ///
  /// Must be >= 0.
  ///
  /// When set to 0, a hairline stroking will be used.
  ///
  /// Default: 1.0
  pub width: f32,

  /// The limit at which a sharp corner is drawn beveled.
  ///
  /// Default: 4.0
  pub miter_limit: f32,

  /// A stroke line cap.
  ///
  /// Default: Butt
  pub line_cap: LineCap,

  /// A stroke line join.
  ///
  /// Default: Miter
  pub line_join: LineJoin,
}

/// Draws at the beginning and end of an open path contour.
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum LineCap {
  /// No stroke extension.
  #[default]
  Butt,
  /// Adds circle.
  Round,
  /// Adds square.
  Square,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum LineJoin {
  /// Extends to miter limit.
  #[default]
  Miter,
  /// Adds circle.
  Round,
  /// Connects outside edges.
  Bevel,
}

/// A path segment.
#[derive(Copy, Clone, PartialEq, Deserialize, Serialize, Debug)]
pub enum PathSegment {
  MoveTo(Point),
  LineTo(Point),
  QuadTo(Point, Point),
  CubicTo(Point, Point, Point),
  Close,
}

/// The radius of each corner of a rounded rectangle.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct Radius {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_left: f32,
  pub bottom_right: f32,
}

#[cfg(feature = "tessellation")]
pub type VertexBuffers<V> = lyon_tessellation::VertexBuffers<Vertex<V>, u32>;

#[cfg(feature = "tessellation")]
#[derive(Copy, Clone, Debug, zerocopy::AsBytes, Default)]
#[repr(packed)]
pub struct Vertex<Attr> {
  pub pos: [f32; 2],
  pub attr: Attr,
}

impl Path {
  #[inline]
  pub fn builder() -> PathBuilder { PathBuilder::default() }

  /// create a rect path.
  pub fn rect(rect: &Rect) -> Self {
    let mut builder = Path::builder();
    builder.rect(rect);
    builder.build()
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  /// #[inline]
  pub fn rect_round(rect: &Rect, radius: &Radius) -> Self {
    let mut builder = Path::builder();
    builder.rect_round(rect, radius);
    builder.build()
  }

  /// create a circle path.
  pub fn circle(center: Point, radius: f32) -> Self {
    let mut builder = Path::builder();
    builder.circle(center, radius);
    builder.build()
  }

  /// Convert this path to a stroked path
  ///
  /// `ts` is the current transform of the path pre applied. Provide it have a
  /// more precise convert.
  pub fn stroke(&self, options: &StrokeOptions, ts: Option<&Transform>) -> Option<Path> {
    stroke_path(&self.0, options, ts).map(Path)
  }

  /// Returns a transformed path in place.
  ///
  /// Some points may become NaN/inf therefore this method can fail.
  pub fn transform(self, ts: &Transform) -> Self {
    let ts: &lyon_algorithms::geom::Transform<f32> = unsafe { std::mem::transmute(ts) };
    Self(self.0.transformed(ts))
  }

  #[cfg(feature = "tessellation")]
  pub fn tessellate<Attr>(
    &self,
    tolerance: f32,
    buffer: &mut VertexBuffers<Attr>,
    vertex_ctor: impl Fn(Point) -> Vertex<Attr>,
  ) {
    use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex};

    let mut fill_tess = FillTessellator::default();
    fill_tess
      .tessellate_path(
        &self.0,
        &FillOptions::non_zero().with_tolerance(tolerance),
        &mut BuffersBuilder::new(buffer, move |v: FillVertex| {
          vertex_ctor(v.position().cast_unit())
        }),
      )
      .unwrap();
  }
}

impl Radius {
  #[inline]
  pub fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> Radius {
    Radius {
      top_left,
      top_right,
      bottom_left,
      bottom_right,
    }
  }

  /// Creates a radius where all radii are radius.
  #[inline]
  pub fn all(radius: f32) -> Radius { Self::new(radius, radius, radius, radius) }

  #[inline]
  pub fn left(left: f32) -> Radius { Self::new(left, 0., left, 0.) }

  #[inline]
  pub fn right(right: f32) -> Radius { Self::new(0., right, 0., right) }

  #[inline]
  pub fn top(top: f32) -> Radius { Self::new(top, top, 0., 0.) }

  #[inline]
  pub fn bottom(bottom: f32) -> Radius { Self::new(0., 0., bottom, bottom) }

  /// Creates a horizontally symmetrical radius where the left and right sides
  /// of the rectangle have the same radii.
  #[inline]
  pub fn horizontal(left: f32, right: f32) -> Radius { Self::new(left, right, left, right) }

  ///Creates a vertically symmetric radius where the top and bottom sides of
  /// the rectangle have the same radii.
  #[inline]
  pub fn vertical(top: f32, bottom: f32) -> Radius { Self::new(top, top, bottom, bottom) }

  #[inline]
  pub fn top_left(top_left: f32) -> Self { Radius { top_left, ..<_>::default() } }

  #[inline]
  pub fn top_right(top_right: f32) -> Self { Radius { top_right, ..<_>::default() } }

  #[inline]
  pub fn bottom_left(bottom_left: f32) -> Self { Radius { bottom_left, ..<_>::default() } }

  #[inline]
  pub fn bottom_right(bottom_right: f32) -> Self { Radius { bottom_right, ..<_>::default() } }
}

impl Default for StrokeOptions {
  fn default() -> Self {
    StrokeOptions {
      width: 1.0,
      miter_limit: 4.0,
      line_cap: LineCap::default(),
      line_join: LineJoin::default(),
    }
  }
}

#[cfg(feature = "tessellation")]
impl<Attr> Vertex<Attr> {
  #[inline]
  pub fn new(pos: [f32; 2], attr: Attr) -> Self { Self { attr, pos } }
}
