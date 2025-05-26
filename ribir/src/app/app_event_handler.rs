use winit::{application::*, event::WindowEvent, event_loop::*};

use crate::app::*;

#[derive(Default)]
pub struct AppHandler {}

impl ApplicationHandler<AppEvent> for AppHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let _guard = active_event_guard(event_loop);
    AppCtx::run_until_stalled();
  }

  fn window_event(
    &mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent,
  ) {
    let wnd_id = new_id(window_id);
    let Some(wnd) = AppCtx::get_window(wnd_id) else {
      return;
    };

    let _guard = active_event_guard(event_loop);

    match event {
      WindowEvent::CloseRequested => {
        AppCtx::remove_wnd(wnd_id);
        if !AppCtx::has_wnd() {
          event_loop.exit();
        }
      }
      WindowEvent::Occluded(false) => {
        // this is triggered before the app re-enters view
        // for example, in something like i3 window manager,
        // when you switch back to the workspace that the app is in
        // in such cases, we need to re-enter the view otherwise the window stays empty
        wnd.draw_frame(true);
      }
      WindowEvent::RedrawRequested => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          // if the window is not visible, don't draw it./
          if wnd.is_visible() != Some(false) {
            // if this frame is really draw, request another redraw. To make sure the draw
            // always end with a empty draw and emit an extra tick cycle message.
            if wnd.draw_frame(false) {
              request_redraw(&wnd);
            }
          }
        }
      }
      WindowEvent::Resized(size) => {
        {
          let mut shell_wnd = wnd.shell_wnd().borrow_mut();
          let ratio = shell_wnd.device_pixel_ratio();
          let size = size.to_logical(ratio as f64);
          shell_wnd.on_resize(Size::new(size.width, size.height));
        }
        request_redraw(&wnd)
      }
      WindowEvent::Focused(focused) => {
        let mut event = AppEvent::WndFocusChanged(wnd_id, focused);

        App::shared()
          .events_stream
          .clone()
          .next(&mut event);
      }
      WindowEvent::KeyboardInput { event, .. } if !wnd.is_pre_editing() => {
        let KeyEvent { physical_key, logical_key, text, location, repeat, state, .. } = event;
        wnd.processes_keyboard_event(physical_key, logical_key, repeat, location, state);
        if state == ElementState::Pressed {
          if let Some(txt) = text {
            wnd.processes_receive_chars(txt.to_string().into());
          }
        }
      }
      WindowEvent::Ime(ime) => App::process_winit_ime_event(&wnd, ime),
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        if state == ElementState::Pressed {
          wnd.force_exit_pre_edit()
        }
        let device_id = Box::new(WinitDeviceId(device_id));
        match state {
          ElementState::Pressed => wnd.process_mouse_press(device_id, button.into()),
          ElementState::Released => wnd.process_mouse_release(device_id, button.into()),
        }
      }
      #[allow(deprecated)]
      event => wnd.processes_native_event(event),
    }
    wnd.emit_events();

    if wnd.need_draw() {
      request_redraw(&wnd)
    }
  }

  fn new_events(&mut self, _: &ActiveEventLoop, _: winit::event::StartCause) {
    Timer::wake_timeout_futures()
  }

  fn user_event(&mut self, _: &ActiveEventLoop, mut event: AppEvent) {
    AppCtx::spawn_local(async move {
      App::shared()
        .events_stream
        .clone()
        .next(&mut event);
    })
    .unwrap();
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let _guard = active_event_guard(event_loop);

    let run_count = AppCtx::run_until_stalled();
    if run_count > 0 {
      for wnd in AppCtx::windows().borrow().values() {
        request_redraw(wnd);
      }
    }
    if run_count > 0 {
      event_loop.set_control_flow(ControlFlow::Poll);
    } else if let Some(t) = Timer::recently_timeout() {
      let control = ControlFlow::wait_duration(t.duration_since(Instant::now()));
      event_loop.set_control_flow(control);
    } else {
      event_loop.set_control_flow(ControlFlow::Wait);
    };
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
