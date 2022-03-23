use crate::prelude::*;
use painter::{StrokeOptions, LineCap, LineJoin};
use path_clean::PathClean;
use std::{env, io, path::PathBuf};
use usvg::Tree;

#[derive(Declare)]
pub struct Svg {
  paths: Box<Vec<Path>>,
  size: Size,
}

const FALLBACK_COLOR: usvg::Color = usvg::Color { red: 0, green: 0, blue: 0 };

impl Svg {
  pub fn new(src: PathBuf) -> Self {
    let file_data = std::fs::read(src.as_path()).unwrap();
    let opt = usvg::Options::default();
    let tree = Tree::from_data(&file_data, &opt.to_ref()).unwrap();

    let svg_size = tree.svg_node().view_box.rect;
    let vb_width = svg_size.width();
    let vb_height = svg_size.height();

    let mut paths = Box::new(vec![]);

    for node in tree.root().descendants() {
      if let usvg::NodeKind::Path(ref p) = *node.borrow() {
        if let Some(ref fill) = p.fill {
          let builder = Svg::build_path(p);
          let fill_color = match fill.paint {
            usvg::Paint::Color(c) => c,
            _ => FALLBACK_COLOR,
          };

          let usvg::Color { red, green, blue } = fill_color;
          let style = Brush::Color(Color {
            red: red as f32,
            green: green as f32,
            blue: blue as f32,
            alpha: 1.,
          });
          paths.push(builder.fill(style));
        }

        if let Some(ref stroke) = p.stroke {
          let builder = Svg::build_path(p);
          let (stroke_color, stroke_opts) = convert_stroke(stroke);
          let usvg::Color { red, green, blue } = stroke_color;
          let style = Brush::Color(Color {
            red: red as f32,
            green: green as f32,
            blue: blue as f32,
            alpha: 1.,
          });
          let width = stroke_opts.line_width;
          paths.push(builder.stroke(width, style))
        }
      }
    }

    let size = Size::new(vb_width as f32, vb_height as f32);

    Self { paths, size }
  }

  fn build_path(p: &usvg::Path) -> Builder {
    let mut builder = Path::builder();
    let mut need_end = false;
    let mut first;
    let mut prev;
    let mut iter = p.data.iter();

    while let Some(segment) = iter.next() {
      match segment {
        usvg::PathSegment::MoveTo { x, y } => {
          if need_end {
            need_end = false;
          } else {
            first = point(x, y);
            need_end = true;
            builder.begin_path(first);
          }
        }
        usvg::PathSegment::LineTo { x, y } => {
          need_end = true;
          prev = point(x, y);
          builder.line_to(prev);
        }
        usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
          need_end = true;
          prev = point(x, y);
          let ctrl1 = point(x1, y1);
          let ctrl2 = point(x2, y2);
          builder.bezier_curve_to(ctrl1, ctrl2, prev);
        }
        usvg::PathSegment::ClosePath => {
          need_end = false;
          builder.close_path();
        }
      }
    }

    builder
  }
}

pub fn load_src(path: impl AsRef<std::path::Path>) -> io::Result<PathBuf> {
  let path = path.as_ref();
  let absolute_path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    env::current_dir()?.join(path)
  }
  .clean();
  Ok(absolute_path)
}

impl RenderWidget for Svg {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let real_size = ctx.box_rect().unwrap().size;

    ctx.painter().scale(
      real_size.width / self.size.width,
      real_size.height / self.size.height,
    );

    self.paths.iter().for_each(|path| {
      ctx.painter().paint_path(path.clone());
    });
  }
}

fn point(x: &f64, y: &f64) -> Point { Point::new((*x) as f32, (*y) as f32) }

fn convert_stroke(s: &usvg::Stroke) -> (usvg::Color, StrokeOptions) {
  let color = match s.paint {
    usvg::Paint::Color(c) => c,
    _ => FALLBACK_COLOR,
  };
  let linecap = match s.linecap {
    usvg::LineCap::Butt => LineCap::Butt,
    usvg::LineCap::Square => LineCap::Square,
    usvg::LineCap::Round => LineCap::Round,
  };
  let linejoin = match s.linejoin {
    usvg::LineJoin::Miter => LineJoin::Miter,
    usvg::LineJoin::Bevel => LineJoin::Bevel,
    usvg::LineJoin::Round => LineJoin::Round,
  };

  let opt = StrokeOptions::tolerance(0.01)
    .with_line_width(s.width.value() as f32)
    .with_line_cap(linecap)
    .with_line_join(linejoin);

  (color, opt)
}
