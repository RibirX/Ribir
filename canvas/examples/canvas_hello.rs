use canvas::{create_canvas_with_render_from_wnd, Color, DeviceSize};
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};
pub type Angle = euclid::Angle<f32>;

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
      let mut layer = canvas.new_2d_layer();
      layer.set_style(Color::YELLOW);

      layer
        .begin_path((0., 70.).into())
        .line_to((100.0, 70.0).into())
        .line_to((100.0, 0.0).into())
        .line_to((250.0, 100.0).into())
        .line_to((100.0, 200.0).into())
        .line_to((100.0, 130.0).into())
        .line_to((0.0, 130.0).into())
        .close_path()
        .fill();

      // layer.rect(100.0, 100.0, 100.0, 100.0).fill();
      // layer.arc(100., 100., 50., Angle::zero(), Angle::pi()).fill();
      canvas.next_frame(&mut render).compose_2d_layer(layer);
    }
    Event::MainEventsCleared => {
      window.request_redraw();
    }
    _ => {}
  });
}
