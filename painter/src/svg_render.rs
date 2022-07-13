use crate::{Brush, Color, LineCap, LineJoin, Path, PathStyle, Point, Size, Transform};
use euclid::approxeq::ApproxEq;
use lyon_tessellation::{math::Point as LyonPoint, path::Path as LyonPath, StrokeOptions};
use palette::FromComponent;
use serde::{Deserialize, Serialize};
use std::{error::Error, io::Read};
use usvg::{Options, Tree};
#[derive(Serialize, Deserialize, Debug)]
pub struct SvgRender {
  pub size: Size,
  pub paths: Vec<SvgRenderPath>,
}

// todo: we need to support currentColor to change svg color.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SvgRenderPath {
  pub path: Path,
  pub transform: Transform,
  pub brush: Option<Brush>,
}

impl SvgRender {
  pub fn from_svg_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    // todo: font_db to parse text
    // But `Options` need own the font db, need a pr for usvg.
    let opt = Options { ..<_>::default() };
    let tree = Tree::from_data(svg_data, &opt.to_ref()).unwrap();
    let svg = tree.svg_node();
    let view_box = svg.view_box.rect;
    let size = Size::new(svg.size.width() as f32, svg.size.height() as f32);
    let scale_x = size.width / view_box.width() as f32;
    let scale_y = size.height / view_box.height() as f32;
    let t = Transform::translation(-view_box.x() as f32, -view_box.y() as f32)
      .then_scale(scale_x, scale_y);

    let mut node = Some(tree.root());
    let mut t_stack = TransformStack::new(t);
    let mut paths = vec![];

    while let Some(ref n) = node {
      use usvg::NodeKind;
      let mut process_children = true;

      match &*n.borrow() {
        NodeKind::Path(p) => {
          t_stack.push(matrix_convert(p.transform));
          if let Some(ref fill) = p.fill {
            let brush = brush_from_usvg_paint(&fill.paint, &tree, fill.opacity);
            let lyon_path = usvg_path_to_lyon_path(p);
            let transform = t_stack.current_transform();
            let path = Path {
              path: lyon_path,
              style: PathStyle::Fill,
            };
            paths.push(SvgRenderPath { path, transform, brush });
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
            let options = StrokeOptions::default()
              .with_line_width(stroke.width.value() as f32)
              .with_line_join(join)
              .with_line_cap(cap);
            let brush = brush_from_usvg_paint(&stroke.paint, &tree, stroke.opacity);
            let lyon_path = usvg_path_to_lyon_path(p);
            let path = Path {
              path: lyon_path,
              style: PathStyle::Stroke(options),
            };
            let transform = t_stack.current_transform();
            paths.push(SvgRenderPath { path, transform, brush });
          }
        }
        NodeKind::Image(_) => {
          // todo;
          log::warn!("[painter]: not support draw embed image in svg, ignored!");
        }
        NodeKind::Group(ref g) => {
          t_stack.push(matrix_convert(g.transform));
          // todo;
          if !g.opacity.value().approx_eq(&1.) {
            log::warn!("[painter]: not support `opacity` in svg, ignored!");
          }
          if g.clip_path.is_some() {
            log::warn!("[painter]: not support `clip path` in svg, ignored!");
          }
          if g.mask.is_some() {
            log::warn!("[painter]: not support `mask` in svg, ignored!");
          }
          if !g.filter.is_empty() {
            log::warn!("[painter]: not support `filters` in svg, ignored!");
          }
        }
        NodeKind::Svg(_) => {
          // svg preprocessed.
        }
        _ => process_children = false,
      }

      if process_children {
        node = n.first_child();
      } else {
        node = None;
      }
      if node.is_none() {
        let mut find_sibling = node;
        while let Some(f) = find_sibling {
          // self node sub-tree paint finished, goto sibling
          t_stack.pop();
          find_sibling = f.next_sibling();
          if find_sibling.is_some() {
            break;
          } else {
            // if there is no more sibling, back to parent to find sibling.
            find_sibling = f.parent();
          }
        }
        node = find_sibling;
      }
    }

    assert_eq!(t_stack.stack.len(), 1);
    Ok(SvgRender { size, paths })
  }

  pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    Self::from_svg_bytes(&bytes)
  }

  pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn Error>> { Ok(bincode::serialize(self)?) }

  pub fn deserialize(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
    Ok(bincode::deserialize(bytes)?)
  }
}

fn usvg_path_to_lyon_path(path: &usvg::Path) -> LyonPath {
  let mut builder = LyonPath::svg_builder();
  path.data.iter().for_each(|seg| match *seg {
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

  builder.build()
}

fn point(x: f64, y: f64) -> LyonPoint { Point::new(x as f32, y as f32).to_untyped() }

fn matrix_convert(t: usvg::Transform) -> Transform {
  let usvg::Transform { a, b, c, d, e, f } = t;
  Transform::new(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
}

fn brush_from_usvg_paint(
  paint: &usvg::Paint,
  tree: &Tree,
  opacity: usvg::Opacity,
) -> Option<Brush> {
  match paint {
    usvg::Paint::Color(usvg::Color { red, green, blue }) => {
      let alpha = u8::from_component(opacity.value());
      let color = Color::new(*red, *green, *blue, alpha);
      Some(Brush::Color(color))
    }
    usvg::Paint::Link(ref id) => {
      // todo
      if let Some(node) = tree.defs_by_id(id) {
        match *node.borrow() {
          usvg::NodeKind::LinearGradient(_) => {
            log::warn!("[painter]: not support `line gradient` in svg, ignored!");
          }
          usvg::NodeKind::RadialGradient(_) => {
            log::warn!("[painter]: not support `radia gradient` in svg, ignored!");
          }
          usvg::NodeKind::Pattern(_) => {
            log::warn!("[painter]: render svg not support `pattern` now , ignored!");
          }
          _ => {}
        }
      }
      None
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

  fn current_transform(&self) -> Transform { self.stack.last().cloned().unwrap() }
}
