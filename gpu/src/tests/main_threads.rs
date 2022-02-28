use painter::{Brush, Color, DeviceSize, PainterBackend, Rect, Size};
use winit::{event_loop::EventLoop, window::WindowBuilder};

use gpu::wgpu_backend_headless;

fn red_img_test<B: PainterBackend>(mut backend: B) {
  let mut painter = painter::Painter::new(1.);
  painter.set_brush(Color::RED);
  painter.rect(&Rect::from_size(Size::new(100., 100.)));
  painter.fill(Brush::Color(Color::RED).into());

  let commands = painter.finish();
  let mut img_size = DeviceSize::zero();
  let mut img_data: Vec<u8> = vec![];
  backend
    .submit(
      commands,
      Some(Box::new(|size, rows| {
        img_size = size;
        rows.for_each(|r| img_data.extend(r))
      })),
    )
    .unwrap();

  let expect_data = std::iter::repeat([255, 0, 0, 255])
    .take(10000)
    .flatten()
    .collect::<Vec<_>>();

  assert_eq!(img_size, DeviceSize::new(100, 100));
  assert_eq!(img_data.len(), expect_data.len());
  assert_eq!(img_data, expect_data);
}

fn headless_smoke() {
  let backend = futures::executor::block_on(wgpu_backend_headless(
    DeviceSize::new(100, 100),
    None,
    None,
    0.01,
    <_>::default(),
  ));

  red_img_test(backend);
}

fn wnd_smoke() {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();
  let backend = futures::executor::block_on(gpu::wgpu_backend_with_wnd(
    &window,
    DeviceSize::new(100, 100),
    None,
    None,
    0.01,
    <_>::default(),
  ));

  red_img_test(backend);
}

fn main() {
  use colored::Colorize;
  
  ribir::test::unit_test_describe! {
    run_unit_test(headless_smoke);
    run_unit_test(wnd_smoke);
  }
}
