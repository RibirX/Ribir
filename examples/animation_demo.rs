/*
use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let style = PathPaintStyle::Stroke(StrokeOptions::default());
  let lyon_path = include_svg!("./Logo.svg");
  let mut paths = vec![];
  lyon_path.paths.into_iter().for_each(|render_path| {
    paths.push(PathPaintKit {
      path: render_path.path,
      brush: render_path.brush.map_or(Brush::Color(Color::BLACK), |b| b),
      style,
    });
  });
  let w = widget! {
    PathsPaintKit {
      anchor: Anchor::left_top(100., 100.),
      transform: Transform::scale(0.5, 0.5),
      id: path_widget,
      paths,
      on_mounted: move |_|  circle_animate.run(),
    }
    Animate {
      id: circle_animate,
      transition: Transition {
        delay: None,
        duration: Duration::from_millis(5000),
        easing: easing::LINEAR,
        repeat: Some(f32::MAX),
      },
      prop: prop!(path_widget.paths, PathPaintKit::paths_lerp_fn(prop!(path_widget.paths))),
      from: vec![
        PathPaintKit {
          path: Path::rect(&Rect::zero()),
          brush: Brush::Color(Color::WHITE),
          style
        }
      ]
    }
  };
  App::run(w);
}
 */

fn main() { panic!("Blocked by path sampler finished.") }
