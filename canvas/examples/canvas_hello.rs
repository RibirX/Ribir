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
      let mut ctx = canvas.get_context_2d();
      let mut frame = canvas.new_screen_frame();
      let mut layer = frame.new_2d_layer();
      layer.set_brush_style(FillStyle::Color(const_color::YELLOW.into()));

      ctx.begin_path(0., 70.);
      ctx.line_to(100.0, 70.0);
      ctx.line_to(100.0, 0.0);
      ctx.line_to(250.0, 100.0);
      ctx.line_to(100.0, 200.0);
      ctx.line_to(100.0, 130.0);
      ctx.line_to(0.0, 130.0);
      ctx.close_path();

      ctx.rect(100.0, 100.0, 100.0, 100.0);

      let path = ctx.get_path();
      layer.fill_path(path);

      frame.compose_2d_layer(layer);
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
