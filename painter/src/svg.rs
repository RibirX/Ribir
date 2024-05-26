use std::{error::Error, io::Read};

use ribir_algo::Resource;
use ribir_geom::{Point, Rect, Size, Transform};
use serde::{Deserialize, Serialize};
use usvg::{Options, Stop, Tree, TreeParsing};

use crate::{
  color::{LinearGradient, RadialGradient},
  Brush, Color, GradientStop, LineCap, LineJoin, PaintCommand, Path, StrokeOptions,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Svg {
  pub size: Size,
  pub commands: Resource<Box<[PaintCommand]>>,
}

/// Fits size into a viewbox. copy from resvg
fn fit_view_box(size: usvg::Size, vb: &usvg::ViewBox) -> usvg::Size {
  let s = vb.rect.size();

  if vb.aspect.align == usvg::Align::None {
    s
  } else if vb.aspect.slice {
    size.expand_to(s)
  } else {
    size.scale_to(s)
  }
}

// todo: we need to support currentColor to change svg color.
impl Svg {
  pub fn parse_from_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    let opt = Options { ..<_>::default() };
    let tree = Tree::from_data(svg_data, &opt).unwrap();
    let view_rect = tree.view_box.rect;
    let size = tree.size;
    let fit_size = fit_view_box(size, &tree.view_box);

    let bound_rect = Rect::from_size(Size::new(f32::MAX, f32::MAX));
    let mut painter = crate::Painter::new(bound_rect);
    painter.apply_transform(
      &Transform::translation(-view_rect.x(), -view_rect.y())
        .then_scale(size.width() / fit_size.width(), size.height() / fit_size.height()),
    );
    tree.root.traverse().for_each(|edge| match edge {
      rctree::NodeEdge::Start(node) => {
        use usvg::NodeKind;
        painter.save();
        match &*node.borrow() {
          NodeKind::Path(p) => {
            painter.apply_transform(&matrix_convert(p.transform));
            let path = usvg_path_to_path(p);
            if let Some(ref fill) = p.fill {
              let (brush, transform) = brush_from_usvg_paint(&fill.paint, fill.opacity, &size);
              let mut painter = painter.save_guard();

              let inverse_ts = transform.inverse().unwrap();
              let path = Resource::new(path.clone().transform(&inverse_ts));
              painter
                .set_brush(brush.clone())
                .apply_transform(&transform)
                .fill_path(path);
              //&o_ts.then(&n_ts.inverse().unwrap())));
            }

            if let Some(ref stroke) = p.stroke {
              let cap = match stroke.linecap {
                usvg::LineCap::Butt => LineCap::Butt,
                usvg::LineCap::Square => LineCap::Square,
                usvg::LineCap::Round => LineCap::Round,
              };
              let join = match stroke.linejoin {
                usvg::LineJoin::Miter => LineJoin::Miter,
                usvg::LineJoin::Bevel => LineJoin::Bevel,
                usvg::LineJoin::Round => LineJoin::Round,
                usvg::LineJoin::MiterClip => LineJoin::MiterClip,
              };
              let options = StrokeOptions {
                width: stroke.width.get(),
                line_cap: cap,
                line_join: join,
                miter_limit: stroke.miterlimit.get(),
              };

              let (brush, transform) = brush_from_usvg_paint(&stroke.paint, stroke.opacity, &size);
              let mut painter = painter.save_guard();

              painter
                .set_brush(brush.clone())
                .apply_transform(&transform);

              let path = path
                .transform(&transform.inverse().unwrap())
                .stroke(&options, Some(painter.get_transform()));

              if let Some(p) = path {
                painter.fill_path(Resource::new(p));
              }
            };
          }
          NodeKind::Image(_) => {
            // todo;
            log::warn!("[painter]: not support draw embed image in svg, ignored!");
          }
          NodeKind::Group(ref g) => {
            painter.apply_transform(&matrix_convert(g.transform));
            // todo;
            if g.opacity.get() != 1. {
              log::warn!("[painter]: not support `opacity` in svg, ignored!");
            }
            if g.clip_path.is_some() {
              log::warn!("[painter]: not support `clip path` in svg, ignored!");
            }
            if g.mask.is_some() {
              log::warn!("[painter]: not support `mask` in svg, ignored!");
            }
            if !g.filters.is_empty() {
              log::warn!("[painter]: not support `filters` in svg, ignored!");
            }
          }
          NodeKind::Text(_) => {
            todo!("Not support text in SVG temporarily, we'll add it after refactoring `painter`.")
          }
        }
      }
      rctree::NodeEdge::End(_) => {
        painter.restore();
      }
    });

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

fn usvg_path_to_path(path: &usvg::Path) -> Path {
  let mut builder = lyon_algorithms::path::Path::svg_builder();
  path.data.segments().for_each(|seg| match seg {
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

fn brush_from_usvg_paint(
  paint: &usvg::Paint, opacity: usvg::Opacity, size: &usvg::Size,
) -> (Brush, Transform) {
  match paint {
    usvg::Paint::Color(usvg::Color { red, green, blue }) => (
      Color::from_rgb(*red, *green, *blue)
        .with_alpha(opacity.get())
        .into(),
      Transform::identity(),
    ),
    usvg::Paint::LinearGradient(linear) => {
      let stops = convert_to_gradient_stops(&linear.stops);
      let size_scale = match linear.units {
        usvg::Units::UserSpaceOnUse => (1., 1.),
        usvg::Units::ObjectBoundingBox => (size.width(), size.height()),
      };
      let gradient = LinearGradient {
        start: Point::new(linear.x1 * size_scale.0, linear.y1 * size_scale.1),
        end: Point::new(linear.x2 * size_scale.0, linear.y2 * size_scale.1),
        stops,
        spread_method: linear.spread_method.into(),
      };

      (Brush::LinearGradient(gradient), matrix_convert(linear.transform))
    }
    usvg::Paint::RadialGradient(radial_gradient) => {
      let stops = convert_to_gradient_stops(&radial_gradient.stops);
      let size_scale = match radial_gradient.units {
        usvg::Units::UserSpaceOnUse => (1., 1.),
        usvg::Units::ObjectBoundingBox => (size.width(), size.height()),
      };
      let gradient = RadialGradient {
        start_center: Point::new(
          radial_gradient.fx * size_scale.0,
          radial_gradient.fy * size_scale.1,
        ),
        start_radius: 0., // usvg not support fr
        end_center: Point::new(
          radial_gradient.cx * size_scale.0,
          radial_gradient.cy * size_scale.1,
        ),
        end_radius: radial_gradient.r.get() * size_scale.0,
        stops,
        spread_method: radial_gradient.spread_method.into(),
      };

      (Brush::RadialGradient(gradient), matrix_convert(radial_gradient.transform))
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
      let usvg::Color { red, green, blue } = stop.color;
      GradientStop {
        offset: stop.offset.get(),
        color: Color::new(red, green, blue, stop.opacity.to_u8()),
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
