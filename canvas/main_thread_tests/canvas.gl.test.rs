#![feature(decl_macro)]

mod test_env;
use canvas::*;
use test_env::{new_canvas, unit_test_describe};

fn env() -> (Canvas, Path) {
  let (canvas, ..) = new_canvas(400, 400);
  let mut path = Path::builder();
  path.add_circle(euclid::Point2D::new(50., 50.), 50., Winding::Positive);
  let path = path.build();

  (canvas, path)
}

fn smoke_draw_circle() {
  let (mut canvas, path, ..) = env();
  let mut frame = canvas.new_texture_frame();
  let mut layer = frame.new_2d_layer();
  layer.fill_path(path);
  frame.compose_2d_layer(layer);

  test_env::assert_frame_eq(
    frame,
    "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
  );
}

fn fill_style_draw() {
  futures::executor::block_on(
    frame.save_as_png(
      "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
    ),
  );
}

fn main() {
  unit_test_describe! {
    run_unit_test(smoke_draw_circle);
    run_unit_test(fill_style_draw);
  }
}
