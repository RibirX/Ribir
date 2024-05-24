use std::ops::Range;

use lyon_algorithms::{
  measure::{PathMeasurements, SampleType},
  path::{Event, Path as LyonPath},
};
use ribir_geom::{Point, Rect, Transform};
use serde::{Deserialize, Serialize};

use crate::path_builder::{stroke_path, PathBuilder};

/// Path widget describe a shape, build the shape from [`Builder`]!
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Path {
  pub(crate) lyon_path: LyonPath,
  // the bounds of the path.
  pub(crate) bounds: Rect,
}

/// Describe how to paint path, fill or stroke.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum PathStyle {
  /// Fill the path.
  #[default]
  Fill,
  /// Stroke path with line width.
  Stroke(StrokeOptions),
}

/// Stroke properties.
#[derive(Clone, Serialize, Deserialize, Debug)]
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
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default, Hash)]
pub enum LineCap {
  /// No stroke extension.
  #[default]
  Butt,
  /// Adds circle.
  Round,
  /// Adds square.
  Square,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default, Hash)]
pub enum LineJoin {
  /// Extends to miter limit.
  #[default]
  Miter,
  /// Extends to miter limit, then clips the corner.
  MiterClip,
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
  QuadTo { ctrl: Point, to: Point },
  CubicTo { to: Point, ctrl1: Point, ctrl2: Point },
  Close(bool),
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

/// Sampler that can queries point at the path or usb-path of the path.
pub struct PathSampler {
  path: LyonPath,
  measurements: PathMeasurements,
}

impl Path {
  #[inline]
  pub fn builder() -> PathBuilder { PathBuilder::default() }

  #[inline]
  pub fn bounds(&self) -> &Rect { &self.bounds }

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
    stroke_path(&self.lyon_path, options, ts).map(Into::into)
  }

  /// Returns a transformed path in place.
  ///
  /// Some points may become NaN/inf therefore this method can fail.
  pub fn transform(self, ts: &Transform) -> Self {
    let ts: &lyon_algorithms::geom::Transform<f32> = unsafe { std::mem::transmute(ts) };
    self.lyon_path.transformed(ts).into()
  }

  /// Create an sampler that can queries point at this path or usb-path of this
  /// path.
  pub fn sampler(&self) -> PathSampler {
    let measurements = PathMeasurements::from_path(&self.lyon_path, 1e-3);
    PathSampler { path: self.lyon_path.clone(), measurements }
  }

  pub fn segments(&self) -> impl Iterator<Item = PathSegment> + '_ {
    self.lyon_path.iter().map(|e| match e {
      Event::Begin { at } => PathSegment::MoveTo(at.cast_unit()),
      Event::Line { to, .. } => PathSegment::LineTo(to.cast_unit()),
      Event::Quadratic { ctrl, to, .. } => {
        PathSegment::QuadTo { ctrl: ctrl.cast_unit(), to: to.cast_unit() }
      }
      Event::Cubic { ctrl1, ctrl2, to, .. } => PathSegment::CubicTo {
        to: to.cast_unit(),
        ctrl1: ctrl1.cast_unit(),
        ctrl2: ctrl2.cast_unit(),
      },
      Event::End { close, .. } => PathSegment::Close(close),
    })
  }

  #[cfg(feature = "tessellation")]
  pub fn tessellate<Attr>(
    &self, tolerance: f32, buffer: &mut VertexBuffers<Attr>,
    vertex_ctor: impl Fn(Point) -> Vertex<Attr>,
  ) {
    use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex};

    let mut fill_tess = FillTessellator::default();
    fill_tess
      .tessellate_path(
        &self.lyon_path,
        &FillOptions::non_zero().with_tolerance(tolerance),
        &mut BuffersBuilder::new(buffer, move |v: FillVertex| {
          vertex_ctor(v.position().cast_unit())
        }),
      )
      .unwrap();
  }
}

impl PathSampler {
  /// Sample point at a given rate along the path.
  #[inline]
  pub fn normalized_sample(&self, rate: f32) -> Point { self.sample(rate, SampleType::Normalized) }

  /// Sample point at a given distance along the path.
  #[inline]
  pub fn distance_sample(&self, dist: f32) -> Point { self.sample(dist, SampleType::Distance) }

  /// Construct a path for a specific rate range of the measured path.
  #[inline]
  pub fn normalized_sub_path(&self, rate_range: Range<f32>) -> Path {
    self.sub_path(rate_range, SampleType::Normalized)
  }

  /// Construct a path for a specific distance range of the measured path.
  #[inline]
  pub fn distance_sub_path(&self, range: Range<f32>) -> Path {
    self.sub_path(range, SampleType::Distance)
  }

  /// Returns the approximate length of the path.
  #[inline]
  pub fn length(&self) -> f32 { self.measurements.length() }

  fn sample(&self, dist: f32, t: SampleType) -> Point {
    let mut sampler = self.measurements.create_sampler(&self.path, t);
    sampler.sample(dist).position().cast_unit()
  }

  fn sub_path(&self, range: Range<f32>, t: SampleType) -> Path {
    let mut sampler = self.measurements.create_sampler(&self.path, t);
    let mut builder = LyonPath::builder();
    sampler.split_range(range, &mut builder);
    builder.build().into()
  }
}

impl Radius {
  #[inline]
  pub const fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> Radius {
    Radius { top_left, top_right, bottom_left, bottom_right }
  }

  /// Creates a radius where all radii are radius.
  #[inline]
  pub const fn all(radius: f32) -> Radius { Self::new(radius, radius, radius, radius) }

  #[inline]
  pub const fn left(left: f32) -> Radius { Self::new(left, 0., left, 0.) }

  #[inline]
  pub const fn right(right: f32) -> Radius { Self::new(0., right, 0., right) }

  #[inline]
  pub const fn top(top: f32) -> Radius { Self::new(top, top, 0., 0.) }

  #[inline]
  pub const fn bottom(bottom: f32) -> Radius { Self::new(0., 0., bottom, bottom) }

  /// Creates a horizontally symmetrical radius where the left and right sides
  /// of the rectangle have the same radii.
  #[inline]
  pub const fn horizontal(left: f32, right: f32) -> Radius { Self::new(left, right, left, right) }

  ///Creates a vertically symmetric radius where the top and bottom sides of
  /// the rectangle have the same radii.
  #[inline]
  pub const fn vertical(top: f32, bottom: f32) -> Radius { Self::new(top, top, bottom, bottom) }

  #[inline]
  pub const fn top_left(top_left: f32) -> Self {
    Radius { top_left, top_right: 0., bottom_left: 0., bottom_right: 0. }
  }

  #[inline]
  pub const fn top_right(top_right: f32) -> Self {
    Radius { top_right, top_left: 0., bottom_left: 0., bottom_right: 0. }
  }

  #[inline]
  pub const fn bottom_left(bottom_left: f32) -> Self {
    Radius { bottom_left, top_left: 0., top_right: 0., bottom_right: 0. }
  }

  #[inline]
  pub const fn bottom_right(bottom_right: f32) -> Self {
    Radius { bottom_right, top_left: 0., top_right: 0., bottom_left: 0. }
  }
}

impl From<LyonPath> for Path {
  fn from(lyon_path: LyonPath) -> Self {
    let bounds = lyon_algorithms::aabb::bounding_box(&lyon_path)
      .to_rect()
      .cast_unit();
    Path { lyon_path, bounds }
  }
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

use ordered_float::OrderedFloat;

impl std::hash::Hash for StrokeOptions {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    OrderedFloat(self.width).hash(state);
    OrderedFloat(self.miter_limit).hash(state);
    self.line_cap.hash(state);
    self.line_join.hash(state);
  }
}

impl PartialEq for StrokeOptions {
  fn eq(&self, other: &Self) -> bool {
    OrderedFloat(self.width).eq(&OrderedFloat(other.width))
      && OrderedFloat(self.miter_limit).eq(&OrderedFloat(other.miter_limit))
      && self.line_cap.eq(&other.line_cap)
      && self.line_join.eq(&other.line_join)
  }
}

impl Eq for StrokeOptions {}

#[cfg(feature = "tessellation")]
impl<Attr> Vertex<Attr> {
  #[inline]
  pub fn new(pos: [f32; 2], attr: Attr) -> Self { Self { attr, pos } }
}
