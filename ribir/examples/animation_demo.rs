#[cfg(any(feature = "crossterm", feature = "winit"))]
fn main() {
  use ribir::prelude::*;
  use ribir::Application;
  use std::time::Duration;

  let style = PathStyle::Stroke(StrokeOptions::default());
  let lyon_path = include_svg!("./Logo.svg");
  let mut paths = vec![];
  lyon_path.paths.into_iter().for_each(|render_path| {
    paths.push(PathPaintKit {
      path: render_path.path,
      brush: render_path.brush.map_or(Brush::Color(Color::BLACK), |b| b),
    });
  });
  let w = widget! {
    PathsPaintKit {
      top_anchor: 100.,
      left_anchor: 100.,
      transform: Transform::scale(0.5, 0.5),
      id: path_widget,
      paths,
      on_mounted: move |_| {
        circle_animate.run();
      },
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
          path: Path::rect(&Rect::zero(), style),
          brush: Brush::Color(Color::WHITE),
        }
      ]
    }
  };

  let mut app = Application::new(material::purple::light());
  let window_builder = app.window_builder(w, Default::default());

  let window_id = app.build_window(window_builder);
  app.exec(window_id);
}

#[cfg(not(any(feature = "crossterm", feature = "winit")))]
fn main() {
  println!("Chose a platform to run:");
  println!("  cargo run --example animation_demo -F winit,wgpu_gl");
  println!("  cargo run --example animation_demo -F crossterm");
}
