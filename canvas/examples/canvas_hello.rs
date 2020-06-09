use canvas::{create_canvas_with_render_from_wnd, Color, DeviceSize, Path, PathBuilder, Winding};
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
  let (mut canvas, mut render) = block_on(create_canvas_with_render_from_wnd(
    &window,
    DeviceSize::new(size.width, size.height),
  ));

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
      let mut layer = canvas.new_2d_layer();
      layer.set_style(Color::YELLOW);
      let mut path = Path::builder();
      path.add_circle(euclid::Point2D::new(200., 200.), 100., Winding::Positive);
      let path = path.build();
      layer.fill_path(path);
      canvas.next_frame(&mut render).compose_2d_layer(layer);
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
