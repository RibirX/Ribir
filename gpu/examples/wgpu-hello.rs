use std::sync::{Arc, RwLock};

use gpu::wgpu_backend_with_wnd;
use painter::{
  image::ColorFormat, Brush, Color, DeviceSize, Painter, PainterBackend, PixelImage, ShallowImage,
  TileMode,
};
use text::{font_db::FontDB, shaper::TextShaper, TypographyStore};
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

fn main() {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_inner_size(winit::dpi::LogicalSize::new(800f32, 600f32))
    .build(&event_loop)
    .unwrap();

  use futures::executor::block_on;

  // Since main can't be async, we're going to need to block
  let size = window.inner_size();
  let font_db = Arc::new(RwLock::new(FontDB::default()));
  font_db.write().unwrap().load_system_fonts();
  let shaper = TextShaper::new(font_db);
  let mut gpu_backend = block_on(wgpu_backend_with_wnd(
    &window,
    DeviceSize::new(size.width, size.height),
    None,
    None,
    0.01,
    shaper,
  ));

  let abs_path =
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("./gpu/examples/leaves.png");
  let decoder = png::Decoder::new(std::fs::File::open(abs_path).unwrap());
  let mut reader = decoder.read_info().unwrap();

  let mut buf = vec![0; reader.output_buffer_size()];
  let info = reader.next_frame(&mut buf).unwrap();

  let data = if info.buffer_size() != buf.len() {
    buf[..info.buffer_size()].to_owned()
  } else {
    buf
  };

  assert_eq!(info.color_type, png::ColorType::Rgba);
  assert_eq!(info.bit_depth, png::BitDepth::Eight);

  let img = PixelImage::new(
    std::borrow::Cow::Owned(data),
    info.width as u16,
    info.height as u16,
    ColorFormat::Rgba8,
  );
  let img = ShallowImage::new(img);

  event_loop.run(move |event, _, control_flow| match event {
    Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
      WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
      WindowEvent::KeyboardInput {
        input:
          KeyboardInput {
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::Escape),
            ..
          },
        ..
      } => *control_flow = ControlFlow::Exit,

      _ => {}
    },
    Event::RedrawRequested(_) => {
      fn draw_arrow_path(painter: &mut Painter) {
        painter
          .begin_path((0., 70.).into())
          .line_to((100.0, 70.0).into())
          .line_to((100.0, 0.0).into())
          .line_to((250.0, 100.0).into())
          .line_to((100.0, 200.0).into())
          .line_to((100.0, 130.0).into())
          .line_to((0.0, 130.0).into())
          .close_path();
      }
      let font_db = Arc::new(RwLock::new(FontDB::default()));
      let store = TypographyStore::new(<_>::default(), font_db.clone(), TextShaper::new(font_db));
      let mut painter = Painter::new(2., store);
      let red_brush = Brush::Color(Color::RED);
      let img_brush = Brush::Image {
        img: img.clone(),
        tile_mode: TileMode::REPEAT_BOTH,
      };

      draw_arrow_path(&mut painter);
      painter.set_brush(red_brush.clone()).fill();

      painter.translate(300., 0.);
      draw_arrow_path(&mut painter);
      painter.set_brush(red_brush).set_line_width(5.).stroke();

      painter.translate(-300., 250.);
      draw_arrow_path(&mut painter);
      painter.set_brush(img_brush.clone()).fill();

      painter.translate(300., 0.);
      draw_arrow_path(&mut painter);
      painter.set_brush(img_brush).set_line_width(25.).stroke();

      let commands = painter.finish();
      gpu_backend.submit(commands, None).unwrap();
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
