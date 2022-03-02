use std::rc::Rc;

use gpu::wgpu_backend_with_wnd;
use painter::{Color, DeviceSize, PainterBackend, Image, ShallowImage, Brush, TileMode, image::ColorFormat};
use text::shaper::TextShaper;
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

#[derive(Clone, Debug)]
pub struct PureColorImage {
  pub size: DeviceSize,
  pub color: Color,
}

impl Image for PureColorImage {
  fn pixel_bytes(&self) -> Box<[u8]> {
    let vec =
      vec![self.color.clone().into_raw(); self.size.area() as usize * 4].into_boxed_slice();
    unsafe { std::mem::transmute(vec) }
  }

  fn size(&self) -> DeviceSize { self.size }

  fn color_format(&self) -> ColorFormat { ColorFormat::Rgba8 }
}

impl PureColorImage {
  pub fn new(color: Color, size: DeviceSize) -> Self { Self { size, color } }

  pub fn shallow_img(color: Color, size: DeviceSize) -> ShallowImage {
    ShallowImage::new(Rc::new(Self::new(color, size)))
  }
}

fn main() {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();

  use futures::executor::block_on;

  // Since main can't be async, we're going to need to block
  let size = window.inner_size();
  let shaper = TextShaper::default();
  shaper.font_db_mut().load_system_fonts();
  let mut gpu_backend = block_on(wgpu_backend_with_wnd(
    &window,
    DeviceSize::new(size.width, size.height),
    None,
    None,
    0.01,
    shaper,
  ));

  let img = PureColorImage::shallow_img(Color::BLUE, DeviceSize::new(1024, 1024));

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
      let mut painter = painter::Painter::new(1.);
      painter.set_brush(Brush::Image {
        img: img.clone(),
        tile_mode: TileMode::COVER_BOTH,
      });
      // painter.set_brush(Color::RED);

      painter
        .begin_path((0., 70.).into())
        .line_to((100.0, 70.0).into())
        .line_to((100.0, 0.0).into())
        .line_to((250.0, 100.0).into())
        .line_to((100.0, 200.0).into())
        .line_to((100.0, 130.0).into())
        .line_to((0.0, 130.0).into())
        .close_path();
      painter.fill(None);

      let commands = painter.finish();
      gpu_backend.submit(commands, None).unwrap();
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
