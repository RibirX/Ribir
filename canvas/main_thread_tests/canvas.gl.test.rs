#![feature(decl_macro)]

mod test_env;
use canvas::*;
use test_env::{new_canvas, unit_test_describe};

fn smoke_draw_circle() {
  let (mut canvas, ..) = new_canvas(400, 400);
  let mut frame = canvas.new_texture_frame();
  let mut layer = frame.new_2d_layer();
  let mut path = Path::builder();
  path.add_circle(euclid::Point2D::new(200., 200.), 100., Winding::Positive);
  let path = path.build();
  layer.fill_path(path);
  frame.compose_2d_layer(layer);

  futures::executor::block_on(
    frame.save_as_png(
      "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
    ),
  );
  // test_env::assert_frame_eq(
  //   frame,
  //   "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
  // );
}

fn main() {
  unit_test_describe! {
    run_unit_test(smoke_draw_circle);
  }
}
