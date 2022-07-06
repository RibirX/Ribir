use painter::{
  text::{font_db::FontDB, shaper::TextShaper, TypographyStore},
  Color, DeviceSize, Painter, PainterBackend, Rect, Size,
};
use std::sync::{Arc, RwLock};

use gpu::wgpu_backend_headless;

fn red_img_test<B: PainterBackend>(mut backend: B) {
  let font_db = Arc::new(RwLock::new(FontDB::default()));
  let store = TypographyStore::new(<_>::default(), font_db.clone(), TextShaper::new(font_db));
  let mut painter = Painter::new(1., store);
  painter
    .set_brush(Color::RED)
    .rect(&Rect::from_size(Size::new(100., 100.)))
    .fill();

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
    TextShaper::new(<_>::default()),
  ));

  red_img_test(backend);
}

#[cfg(feature = "event_loop")]
fn wnd_smoke() {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();
  let backend = futures::executor::block_on(gpu::wgpu_backend_with_wnd(
    &window,
    DeviceSize::new(100, 100),
    None,
    None,
    0.01,
    TextShaper::new(<_>::default()),
  ));

  red_img_test(backend);
}

fn main() {
  use colored::Colorize;

  ribir::test::unit_test_describe! {
    run_unit_test(headless_smoke);
  }

  #[cfg(feature = "ui_window")]
  use winit::{event_loop::EventLoop, window::WindowBuilder};
  ribir::test::unit_test_describe! {}
}
