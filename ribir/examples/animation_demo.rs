use std::time::Duration;
use ribir::prelude::*;

fn main() {
  let style = PathStyle::Stroke(StrokeOptions::default());
  let lyon_path = include_svg!("./ribir-logo.svg");
  let path = lyon_path.paths.get(2).unwrap().path.clone();
  // let path3 = Path::circle(Point::new(100., 100.), 50., style);
  // let path4 = path3.clone();
  let w = widget! {
    Stack {
      Button {
        left_anchor: 10.,
        top_anchor: 10.,
        mounted: move |_| {
          circle_animate.run();
        },
        ButtonText::new("START 2")
      }
      PathWidget {
        id: path_widget,
        path,
        brush: Color::BLACK,
      }
    }
    Animate {
      id: circle_animate,
      transition: Transition {
        delay: None,
        duration: Duration::from_millis(2000),
        easing: easing::LINEAR,
        repeat: Some(f32::MAX),
      },
      prop: prop!(path_widget.path, PathPaintKit::path_lerp_fn(prop!(path_widget.path), style)),
      from: Path::circle(Point::new(100., 100.), 0., style)
    }
  };
  app::run(w);
}
