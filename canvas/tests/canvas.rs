use canvas::*;
use winit::{
  dpi::PhysicalSize,
  event_loop::EventLoop,
  window::{Window, WindowBuilder},
};

pub fn new_canvas(width: u32, height: u32) -> (Canvas, Window, EventLoop<()>) {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();
  window.set_inner_size(PhysicalSize::new(width, height));

  use futures::executor::block_on;
  // Since main can't be async, we're going to need to block
  let size = window.inner_size();
  let canvas = block_on(Canvas::new(&window, size.width, size.height));
  (canvas, window, event_loop)
}

#[test]
fn coordinate_2d_map() {
  let (mut canvas, ..) = new_canvas(400, 400);
  {
    let mut frame = canvas.new_texture_frame();
    let mut layer = frame.new_2d_layer();
    let mut path = Path::builder();
    path.add_circle(euclid::Point2D::new(200., 200.), 100., Winding::Positive);
    let path = path.build();
    layer.fill_path(path);
    frame.compose_2d_layer(layer);
    futures::executor::block_on(
      frame.capture_screenshot(std::fs::File::create("./red.png").unwrap()),
    )
    .unwrap();
  }
  // let lt = matrix.transform_point(Point::new(0., 0.));
  // assert_eq!((lt.x, lt.y), (-1., 1.));

  // let rt = matrix.transform_point(Point::new(400., 0.));
  // assert_eq!((rt.x, rt.y), (1., 1.));

  // let lb = matrix.transform_point(Point::new(0., 400.));
  // assert_eq!((lb.x, lb.y), (-1., -1.));

  // let rb = matrix.transform_point(Point::new(400., 400.));
  // assert_eq!((rb.x, rb.y), (1., -1.));
}
