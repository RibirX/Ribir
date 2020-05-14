#![feature(decl_macro)]

mod test_env;
use canvas::*;
use test_env::{assert_frame_eq, new_canvas, unit_test_describe};

fn env() -> (Canvas, Path) {
  let (canvas, ..) = new_canvas(400, 400);
  let mut path = Path::builder();
  path.add_circle(euclid::Point2D::new(0., 0.), 50., Winding::Positive);
  let path = path.build();

  (canvas, path)
}

fn smoke_draw_circle() {
  let (mut canvas, path, ..) = env();

  let mut frame = canvas.new_texture_frame();
  let mut layer = frame.new_2d_layer();
  layer.set_brush_style(FillStyle::Color(const_color::BLACK.into()));
  layer.translate(50., 50.);
  layer.fill_path(path);
  frame.compose_2d_layer(layer);

  assert_frame_eq!(
    frame,
    "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
  );
}

fn color_palette_texture() {
  let (mut canvas, path, ..) = env();
  {
    let mut frame = canvas.new_texture_frame();
    let mut layer = frame.new_2d_layer();

    let mut fill_color_circle = |color: Color, offset_x: f32, offset_y: f32| {
      layer
        .set_brush_style(FillStyle::Color(color))
        .translate(offset_x, offset_y)
        .fill_path(path.clone());
    };

    fill_color_circle(const_color::YELLOW.into(), 50., 50.);
    fill_color_circle(const_color::RED.into(), 100., 0.);
    fill_color_circle(const_color::PINK.into(), 100., 0.);
    fill_color_circle(const_color::GREEN.into(), 100., 0.);
    fill_color_circle(const_color::BLUE.into(), -0., 100.);

    frame.compose_2d_layer(layer);

    assert_frame_eq!(
      frame,
      "./main_thread_tests/test_imgs/canvas/color_palette_texture.png",
    );
  }

  // canvas.log_texture_atlas();
}

fn main() {
  unit_test_describe! {
    run_unit_test(smoke_draw_circle);
    run_unit_test(color_palette_texture);
  }
}
