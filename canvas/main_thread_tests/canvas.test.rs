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

  test_env::assert_frame_eq(
    frame,
    "./main_thread_tests/test_imgs/canvas/smoke_draw_circle.png",
  );
}

fn coordinate_2d_to_wgpu() {
  let (canvas, ..) = new_canvas(400, 400);
  let matrix = canvas.coordinate_2d_to_device_matrix();

  let lt = matrix.transform_point(Point::new(0., 0.));
  assert_eq!((lt.x, lt.y), (-1., 1.));

  let rt = matrix.transform_point(Point::new(400., 0.));
  assert_eq!((rt.x, rt.y), (1., 1.));

  let lb = matrix.transform_point(Point::new(0., 400.));
  assert_eq!((lb.x, lb.y), (-1., -1.));

  let rb = matrix.transform_point(Point::new(400., 400.));
  assert_eq!((rb.x, rb.y), (1., -1.0));
}

fn main() {
  unit_test_describe! {
    run_unit_test(coordinate_2d_to_wgpu);
    run_unit_test(smoke_draw_circle);
  }
}
