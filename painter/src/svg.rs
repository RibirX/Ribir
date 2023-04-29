use crate::{
  Brush, Color, LineCap, LineJoin, Path, PathPaintStyle, Point, Size, StrokeOptions, Transform,
};
use euclid::approxeq::ApproxEq;
use palette::FromComponent;
use serde::{Deserialize, Serialize};
use std::{error::Error, io::Read};
use usvg::{Options, Tree, TreeParsing};

#[derive(Serialize, Deserialize)]
pub struct Svg {
  pub size: Size,
  pub paths: Box<[SvgPath]>,
}

#[derive(Serialize, Deserialize)]
pub struct SvgPath {
  pub path: Path,
  pub brush: Brush,
  pub style: PathPaintStyle,
  pub transform: Transform,
}

// todo: we need to support currentColor to change svg color.
impl Svg {
  pub fn parse_from_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    let opt = Options { ..<_>::default() };
    let tree = Tree::from_data(svg_data, &opt).unwrap();
    let view_rect = tree.view_box.rect;
    let size = tree.size;
    let fit_size = size.fit_view_box(&tree.view_box);
    let scale_x = size.width() / fit_size.width();
    let scale_y = size.height() / fit_size.height();
    let t = Transform::translation(-view_rect.x() as f32, -view_rect.y() as f32)
      .then_scale(scale_x as f32, scale_y as f32);

    let mut t_stack = TransformStack::new(t);
    let mut paths = vec![];

    tree.root.traverse().for_each(|edge| match edge {
      rctree::NodeEdge::Start(node) => {
        use usvg::NodeKind;

        match &*node.borrow() {
          NodeKind::Path(p) => {
            t_stack.push(matrix_convert(p.transform));
            let path = usvg_path_to_path(p);
            let transform = t_stack.current_transform().clone();
            if let Some(ref fill) = p.fill {
              let brush = brush_from_usvg_paint(&fill.paint, fill.opacity);
              paths.push(SvgPath {
                path: path.clone(),
                brush,
                transform: transform.clone(),
                style: PathPaintStyle::Fill,
              });
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
              };
              let options = StrokeOptions {
                width: stroke.width.get() as f32,
                line_cap: cap,
                line_join: join,
                miter_limit: stroke.miterlimit.get() as f32,
              };

              let brush = brush_from_usvg_paint(&stroke.paint, stroke.opacity);
              paths.push(SvgPath {
                path,
                brush,
                transform,
                style: PathPaintStyle::Stroke(options),
              });
            };
          }
          NodeKind::Image(_) => {
            // todo;
            log::warn!("[painter]: not support draw embed image in svg, ignored!");
          }
          NodeKind::Group(ref g) => {
            t_stack.push(matrix_convert(g.transform));
            // todo;
            if !g.opacity.get().approx_eq(&1.) {
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
        t_stack.pop();
      }
    });

    Ok(Svg {
      size: Size::new(size.width() as f32, size.height() as f32),
      paths: paths.into_boxed_slice(),
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
    usvg::PathSegment::MoveTo { x, y } => {
      builder.move_to(point(x, y));
    }
    usvg::PathSegment::LineTo { x, y } => {
      builder.line_to(point(x, y));
    }
    usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
      builder.cubic_bezier_to(point(x1, y1), point(x2, y2), point(x, y));
    }
    usvg::PathSegment::ClosePath => builder.close(),
  });

  builder.build().into()
}

fn point(x: f64, y: f64) -> lyon_algorithms::math::Point {
  Point::new(x as f32, y as f32).to_untyped()
}

fn matrix_convert(t: usvg::Transform) -> Transform {
  let usvg::Transform { a, b, c, d, e, f } = t;
  Transform::new(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}

fn brush_from_usvg_paint(paint: &usvg::Paint, opacity: usvg::Opacity) -> Brush {
  match paint {
    usvg::Paint::Color(usvg::Color { red, green, blue }) => {
      let alpha = u8::from_component(opacity.get());
      Color::new(*red, *green, *blue, alpha).into()
    }
    paint => {
      log::warn!("[painter]: not support `{paint:?}` in svg, use black instead!");
      Color::BLACK.into()
    }
  }
}

struct TransformStack {
  stack: Vec<Transform>,
}

impl TransformStack {
  fn new(t: Transform) -> Self { TransformStack { stack: vec![t] } }

  fn push(&mut self, mut t: Transform) {
    if let Some(p) = self.stack.last() {
      t = p.then(&t);
    }
    self.stack.push(t);
  }

  fn pop(&mut self) -> Option<Transform> { self.stack.pop() }

  fn current_transform(&self) -> &Transform { self.stack.last().unwrap() }
}
