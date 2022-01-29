use gpu::wgpu_backend_with_wnd;
use painter::{Color, DeviceSize, PainterBackend};
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

fn main() {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop).unwrap();

  use futures::executor::block_on;

  // Since main can't be async, we're going to need to block
  let size = window.inner_size();
  let mut gpu_backend = block_on(wgpu_backend_with_wnd(
    &window,
    DeviceSize::new(size.width, size.height),
    None,
    None,
    0.01,
    <_>::default(),
  ));

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
      painter.set_brush(Color::YELLOW);

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
      gpu_backend.submit(commands);
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
