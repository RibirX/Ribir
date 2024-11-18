use std::{error::Error, io::Read, vec};

use ribir_algo::Resource;
use ribir_geom::{Point, Rect, Size, Transform};
use serde::{Deserialize, Serialize};
use usvg::{Options, Stop, Tree};

use crate::{
  Brush, Color, GradientStop, LineCap, LineJoin, PaintCommand, Path, StrokeOptions,
  color::{LinearGradient, RadialGradient},
};

/// This is a basic SVG support designed for rendering to Ribir painter. It is
/// currently quite simple and primarily used for Ribir icons. More features
/// will be added as needed.

#[derive(Serialize, Deserialize, Clone)]
pub struct Svg {
  pub size: Size,
  pub commands: Resource<Box<[PaintCommand]>>,
}

// todo: we need to support currentColor to change svg color.
// todo: share fontdb
impl Svg {
  pub fn parse_from_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    let opt = Options { ..<_>::default() };
    let tree = Tree::from_data(svg_data, &opt).unwrap();

    let size = tree.size();

    let bound_rect = Rect::from_size(Size::new(f32::MAX, f32::MAX));
    let mut painter = crate::Painter::new(bound_rect);
    paint_group(tree.root(), &mut painter);

    let paint_commands = painter.finish().to_owned().into_boxed_slice();

    Ok(Svg {
      size: Size::new(size.width(), size.height()),
      commands: Resource::new(paint_commands),
    })
  }

  pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    Self::parse_from_bytes(&bytes)
  }

  pub fn serialize(&self) -> Result<String, Box<dyn Error>> {
    // use json replace bincode, because https://github.com/Ogeon/palette/issues/130
    Ok(serde_json::to_string(self)?)
  }

  pub fn deserialize(str: &str) -> Result<Self, Box<dyn Error>> { Ok(serde_json::from_str(str)?) }
}

fn paint_group(g: &usvg::Group, painter: &mut crate::Painter) {
  let mut painter = painter.save_guard();
  for child in g.children() {
    match child {
      usvg::Node::Group(g) => {
        // todo;
        painter.apply_alpha(g.opacity().get());

        if g.clip_path().is_some() {
          log::warn!("[painter]: not support `clip path` in svg, ignored!");
        }
        if g.mask().is_some() {
          log::warn!("[painter]: not support `mask` in svg, ignored!");
        }
        if !g.filters().is_empty() {
          log::warn!("[painter]: not support `filters` in svg, ignored!");
        }
        paint_group(g, &mut painter);
      }
      usvg::Node::Path(p) => {
        painter.set_transform(matrix_convert(p.abs_transform()));
        let path = usvg_path_to_path(p);
        if let Some(fill) = p.fill() {
          let (brush, transform) = brush_from_usvg_paint(fill.paint(), fill.opacity());

          let inverse_ts = transform.inverse().unwrap();
          let path = Resource::new(path.clone().transform(&inverse_ts));
          painter
            .set_fill_brush(brush.clone())
            .apply_transform(&transform)
            .fill_path(path.into());
          //&o_ts.then(&n_ts.inverse().unwrap())));
        }

        if let Some(stroke) = p.stroke() {
          let options = StrokeOptions {
            width: stroke.width().get(),
            line_cap: stroke.linecap().into(),
            line_join: stroke.linejoin().into(),
            miter_limit: stroke.miterlimit().get(),
          };

          let (brush, transform) = brush_from_usvg_paint(stroke.paint(), stroke.opacity());
          painter
            .set_stroke_brush(brush.clone())
            .apply_transform(&transform);

          let path = path
            .transform(&transform.inverse().unwrap())
            .stroke(&options, Some(painter.transform()));

          if let Some(p) = path {
            painter.fill_path(Resource::new(p).into());
          }
        };
      }
      usvg::Node::Image(_) => {
        // todo;
        log::warn!("[painter]: not support draw embed image in svg, ignored!");
      }
      usvg::Node::Text(t) => paint_group(t.flattened(), &mut painter),
    }
  }
}
fn usvg_path_to_path(path: &usvg::Path) -> Path {
  let mut builder = lyon_algorithms::path::Path::svg_builder();
  path.data().segments().for_each(|seg| match seg {
    usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
      builder.move_to(point(pt.x, pt.y));
    }
    usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
      builder.line_to(point(pt.x, pt.y));
    }
    usvg::tiny_skia_path::PathSegment::CubicTo(pt1, pt2, pt3) => {
      builder.cubic_bezier_to(point(pt1.x, pt1.y), point(pt2.x, pt2.y), point(pt3.x, pt3.y));
    }
    usvg::tiny_skia_path::PathSegment::QuadTo(pt1, pt2) => {
      builder.quadratic_bezier_to(point(pt1.x, pt1.y), point(pt2.x, pt2.y));
    }
    usvg::tiny_skia_path::PathSegment::Close => builder.close(),
  });

  builder.build().into()
}

fn point(x: f32, y: f32) -> lyon_algorithms::math::Point { Point::new(x, y).to_untyped() }

fn matrix_convert(t: usvg::Transform) -> Transform {
  let usvg::Transform { sx, kx, ky, sy, tx, ty } = t;
  Transform::new(sx, ky, kx, sy, tx, ty)
}

fn brush_from_usvg_paint(paint: &usvg::Paint, opacity: usvg::Opacity) -> (Brush, Transform) {
  match paint {
    usvg::Paint::Color(usvg::Color { red, green, blue }) => (
      Color::from_rgb(*red, *green, *blue)
        .with_alpha(opacity.get())
        .into(),
      Transform::identity(),
    ),
    usvg::Paint::LinearGradient(linear) => {
      let stops = convert_to_gradient_stops(linear.stops());
      let gradient = LinearGradient {
        start: Point::new(linear.x1(), linear.y1()),
        end: Point::new(linear.x2(), linear.y2()),
        stops,
        spread_method: linear.spread_method().into(),
      };

      (Brush::LinearGradient(gradient), matrix_convert(linear.transform()))
    }
    usvg::Paint::RadialGradient(radial_gradient) => {
      let stops = convert_to_gradient_stops(radial_gradient.stops());
      let gradient = RadialGradient {
        start_center: Point::new(radial_gradient.fx(), radial_gradient.fy()),
        start_radius: 0., // usvg not support fr
        end_center: Point::new(radial_gradient.cx(), radial_gradient.cy()),
        end_radius: radial_gradient.r().get(),
        stops,
        spread_method: radial_gradient.spread_method().into(),
      };

      (Brush::RadialGradient(gradient), matrix_convert(radial_gradient.transform()))
    }
    paint => {
      log::warn!("[painter]: not support `{paint:?}` in svg, use black instead!");
      (Color::BLACK.into(), Transform::identity())
    }
  }
}

fn convert_to_gradient_stops(stops: &[Stop]) -> Vec<GradientStop> {
  assert!(!stops.is_empty());

  let mut stops: Vec<_> = stops
    .iter()
    .map(|stop| {
      let usvg::Color { red, green, blue } = stop.color();
      GradientStop {
        offset: stop.offset().get(),
        color: Color::new(red, green, blue, stop.opacity().to_u8()),
      }
    })
    .collect();

  stops.sort_by(|s1, s2| s1.offset.partial_cmp(&s2.offset).unwrap());

  if let Some(first) = stops.first() {
    if first.offset != 0. {
      stops.insert(0, GradientStop { offset: 0., color: first.color });
    }
  }
  if let Some(last) = stops.last() {
    if last.offset < 1. {
      stops.push(GradientStop { offset: 1., color: last.color });
    }
  }
  stops
}

impl From<usvg::LineCap> for LineCap {
  fn from(value: usvg::LineCap) -> Self {
    match value {
      usvg::LineCap::Butt => LineCap::Butt,
      usvg::LineCap::Round => LineCap::Round,
      usvg::LineCap::Square => LineCap::Square,
    }
  }
}

impl From<usvg::LineJoin> for LineJoin {
  fn from(value: usvg::LineJoin) -> Self {
    match value {
      usvg::LineJoin::Miter => LineJoin::Miter,
      usvg::LineJoin::MiterClip => LineJoin::MiterClip,
      usvg::LineJoin::Round => LineJoin::Round,
      usvg::LineJoin::Bevel => LineJoin::Bevel,
    }
  }
}
