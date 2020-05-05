use canvas::*;
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
  let mut canvas = block_on(Canvas::new(&window, size.width, size.height));

  event_loop.run(move |event, _, control_flow| match event {
    Event::WindowEvent {
      ref event,
      window_id,
    } if window_id == window.id() => match event {
      WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
      WindowEvent::KeyboardInput { input, .. } => match input {
        KeyboardInput {
          state: ElementState::Pressed,
          virtual_keycode: Some(VirtualKeyCode::Escape),
          ..
        } => *control_flow = ControlFlow::Exit,
        _ => {}
      },
      _ => {}
    },
    Event::RedrawRequested(_) => {
      let mut frame = canvas.new_frame();
      let mut layer = frame.new_2d_layer();
      let mut path = Path::builder();
      path.add_circle(Point::new(0.1, 0.1), 0.3, Winding::Positive);
      let path = path.build();
      layer.fill_path(path);
      frame.compose_2d_layer(layer);
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
