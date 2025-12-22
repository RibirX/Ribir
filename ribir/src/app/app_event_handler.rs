use ribir_core::window::UiEvent;
use winit::{
  application::*,
  event::{ElementState, MouseScrollDelta, WindowEvent},
  event_loop::*,
};

use crate::app::*;

#[derive(Default)]
pub struct AppHandler {}

impl ApplicationHandler<RibirAppEvent> for AppHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let _guard = active_event_guard(event_loop);
    App::pump_ui_tasks();
  }

  fn window_event(
    &mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent,
  ) {
    let wnd_id = new_id(window_id);
    if App::shell_window(wnd_id).is_none() {
      return;
    };

    let _guard = active_event_guard(event_loop);

    event_loop.set_control_flow(ControlFlow::Wait);

    match event {
      WindowEvent::CloseRequested => {
        App::send_event(UiEvent::CloseRequest { wnd_id });
      }
      WindowEvent::Occluded(false) => {
        // this is triggered before the app re-enters view
        // for example, in something like i3 window manager,
        // when you switch back to the workspace that the app is in
        // in such cases, we need to re-enter the view otherwise the window stays empty
        App::send_event(UiEvent::RedrawRequest { wnd_id, force: true });
      }
      WindowEvent::RedrawRequested => {
        // if the window is not visible, don't draw it./
        App::send_event(UiEvent::RedrawRequest { wnd_id, force: false });
      }
      WindowEvent::Resized(size) => {
        if let Some(shell_wnd) = App::shell_window(wnd_id) {
          let device_size = DeviceSize::new(size.width as i32, size.height as i32);
          shell_wnd.borrow_mut().on_resize(device_size);
          let ratio = shell_wnd.borrow().winit_wnd.scale_factor();
          let size = size.to_logical(ratio);
          App::send_event(UiEvent::Resize { wnd_id, size: Size::new(size.width, size.height) });
          shell_wnd.borrow().winit_wnd.request_redraw();
        }
      }
      WindowEvent::Focused(focused) => {
        let mut event = AppEvent::WndFocusChanged(wnd_id, focused);

        App::shared()
          .events_stream
          .clone()
          .next(&mut event);
      }
      WindowEvent::KeyboardInput { event, .. } => {
        App::send_event(UiEvent::KeyBoard {
          wnd_id,
          key: event.logical_key,
          physical_key: event.physical_key,
          state: event.state,
          is_repeat: event.repeat,
          location: event.location,
        });
        if event.state == ElementState::Pressed
          && let Some(txt) = event.text
        {
          App::send_event(UiEvent::ReceiveChars { wnd_id, chars: txt.to_string().into() });
        }
      }
      WindowEvent::Ime(ime) => {
        App::send_event(UiEvent::ImePreEdit { wnd_id, ime });
      }
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        let device_id = Box::new(WinitDeviceId(device_id));
        App::send_event(UiEvent::MouseInput { wnd_id, device_id, button: button.into(), state });
      }
      WindowEvent::ModifiersChanged(s) => {
        App::send_event(UiEvent::ModifiersChanged { wnd_id, state: s.state() });
      }
      WindowEvent::CursorMoved { position, .. } => {
        if let Some(shell_wnd) = App::shell_window(wnd_id) {
          let ratio = shell_wnd.borrow().winit_wnd.scale_factor();
          let pos = position.to_logical::<f32>(ratio);
          App::send_event(UiEvent::CursorMoved { wnd_id, pos: Point::new(pos.x, pos.y) });
        }
      }
      WindowEvent::CursorLeft { .. } => {
        App::send_event(UiEvent::CursorLeft { wnd_id });
      }
      WindowEvent::MouseWheel { delta, .. } => {
        if let Some(shell_wnd) = App::shell_window(wnd_id) {
          let wnd_factor = shell_wnd.borrow().winit_wnd.scale_factor();
          let (delta_x, delta_y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (x * 16., y * 16.),
            MouseScrollDelta::PixelDelta(delta) => {
              let winit::dpi::LogicalPosition { x, y } = delta.to_logical(wnd_factor);
              (x, y)
            }
          };
          App::send_event(UiEvent::MouseWheel { wnd_id, delta_x, delta_y });
        }
      }
      _ => (),
    }
  }

  fn new_events(&mut self, _: &ActiveEventLoop, _: winit::event::StartCause) {}

  fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RibirAppEvent) {
    let _guard = active_event_guard(event_loop);
    match event {
      RibirAppEvent::App(mut e) => {
        App::shared().events_stream.clone().next(&mut e);
      }
      RibirAppEvent::Cmd(cmd) => match cmd {
        ShellCmd::RunAsync { fut } => {
          App::spawn_local(fut);
        }
        ShellCmd::Exit => {
          event_loop.exit();
        }
        _ => {
          if let Some(wnd) = App::shell_window(cmd.wnd_id().unwrap()) {
            wnd.borrow_mut().deal_cmd(cmd);
          }
        }
      },
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let _guard = active_event_guard(event_loop);
    App::pump_ui_tasks();
  }
}

fn active_event_guard(active: &ActiveEventLoop) -> impl Drop {
  struct Guard;
  let mut event_loop = App::shared().event_loop.borrow_mut();
  if let Some(event_loop) = event_loop.as_mut() {
    assert!(
      matches!(event_loop, EventLoopState::Running(_)),
      "Try to guard a non-running event loop"
    );
  } else {
    let active: &'static ActiveEventLoop = unsafe { std::mem::transmute(active) };
    *event_loop = Some(EventLoopState::Running(active));
  }

  impl Drop for Guard {
    fn drop(&mut self) {
      let mut event_loop = App::shared().event_loop.borrow_mut();
      assert!(event_loop.is_some(), "event loop is not set");
      event_loop.take();
    }
  }

  Guard
}

#[derive(Debug, Clone, Copy)]
struct WinitDeviceId(winit::event::DeviceId);

impl DeviceId for WinitDeviceId {
  fn as_any(&self) -> &dyn std::any::Any { self }

  fn is_same_device(&self, other: &dyn DeviceId) -> bool {
    other
      .as_any()
      .downcast_ref::<WinitDeviceId>()
      .is_some_and(|other| self.0 == other.0)
  }

  fn clone_boxed(&self) -> Box<dyn DeviceId> { Box::new(WinitDeviceId(self.0)) }
}
