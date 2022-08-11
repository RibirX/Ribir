use ribir::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

fn wheel_widget(w: Widget) {
  let mut wnd = Window::without_render(w, Size::new(100., 100.));

  wnd.draw_frame();
  let device_id = unsafe { DeviceId::dummy() };
  wnd.processes_native_event(WindowEvent::MouseWheel {
    device_id,
    delta: MouseScrollDelta::LineDelta(1.0, 1.0),
    phase: TouchPhase::Started,
    modifiers: ModifiersState::default(),
  });
}

#[test]
fn listener_trigger_have_handler() {
  let handler_call_times = Rc::new(Cell::new(0));
  let h1 = handler_call_times.clone();
  let mut animate;
  let w = widget! {
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
      // tricky: leak `leak_animate` to outside `widget!`s scope, just for test.
      background: {
        animate = leak_animate.clone();
        Color::RED
      },
      on_wheel: move |_| h1.set(h1.get() + 1),
    }
    animations {
      sized_box.on_wheel: Animate {
        id: leak_animate,
        from: State {
          sized_box.size: Size::zero(),
        },
        transition: Transition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(100)
        }
      }
    }
  };

  wheel_widget(w);

  assert!(animate.raw_ref().is_running());
  assert_eq!(handler_call_times.get(), 1);
}

#[test]
fn listener_trigger() {
  let animate;
  let w = widget! {
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
      // tricky: leak `leak_animate` to outside `widget!`s scope, just for test.
      background: {
        animate = leak_animate.clone();
        Color::RED
      }
    }
    animations {
      sized_box.on_wheel: Animate {
        id: leak_animate,
        from: State {
          sized_box.size: Size::zero(),
        },
        transition: Transition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(100)
        }
      }
    }
  };

  wheel_widget(w);

  assert!(animate.raw_ref().is_running());
}
