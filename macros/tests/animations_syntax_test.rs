use ribir::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

fn wheel_widget(w: Widget) -> Window {
  let mut wnd = Window::default_mock(w, None);

  wnd.draw_frame();
  let device_id = unsafe { DeviceId::dummy() };
  wnd.processes_native_event(WindowEvent::MouseWheel {
    device_id,
    delta: MouseScrollDelta::LineDelta(1.0, 1.0),
    phase: TouchPhase::Started,
    modifiers: ModifiersState::default(),
  });
  wnd
}

#[test]
fn listener_trigger_have_handler() {
  let handler_call_times = Rc::new(Cell::new(0));
  let h1 = handler_call_times.clone();
  let animate_state = Stateful::new(false);

  let w = widget! {
    states { animate_state:  animate_state.clone() }
    init ctx => {
      let linear_transition = transitions::LINEAR.of(ctx);
    }
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
      wheel: move |_| {
        h1.set(h1.get() + 1);
        leak_animate.run();
      },
    }
    Animate {
      id: leak_animate,
      transition: linear_transition,
      prop: prop!(sized_box.size),
      from: ZERO_SIZE,
    }
    finally {
      watch!(leak_animate.is_running())
        .subscribe(move |v| *animate_state = v);
    }
  };

  wheel_widget(w);

  assert!(*animate_state.state_ref());
  assert_eq!(handler_call_times.get(), 1);
}

#[test]
fn listener_trigger() {
  let animate_state = Stateful::new(false);

  let w = widget! {
    states { animate_state:  animate_state.clone() }
    init ctx => {
      let linear_transition = Transition::declare_builder()
      .easing(easing::LINEAR)
      .duration(Duration::from_millis(100))
      .build(ctx);
    }
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
      wheel: move |_| leak_animate.run()
    }
    Animate {
      id: leak_animate,
      prop: prop!(sized_box.size),
      from: ZERO_SIZE,
      transition: linear_transition,
    }
    finally {
      watch!(leak_animate.is_running())
        .subscribe(move |v| *animate_state = v);
    }
  };

  wheel_widget(w);

  assert!(*animate_state.state_ref());
}
