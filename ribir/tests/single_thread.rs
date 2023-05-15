use ribir::timer::new_timer;
use rxrust::scheduler::NEW_TIMER_FN;

mod test_single_thread {
  use futures::executor::LocalPool;
  use ribir::timer::wake_timeout_futures;
  use ribir_core::test::{default_mock_window, mock_window};
  use std::{cell::RefCell, rc::Rc};
  use std::{thread::sleep, time::Duration};
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton, WindowEvent};

  use ribir_core::{
    prelude::*,
    test::{assert_layout_result, ExpectRect, MockBox},
  };

  pub fn test_widget_with_timer() {
    let w = widget! {
      MockBox {
        id: c,
        size: Size::new(20., 20.)
      }
      finally ctx => {
        observable::of(Size::new(10., 10.))
          .delay(Duration::from_millis(10), ctx.wnd_ctx().frame_scheduler())
          .subscribe(move |v| c.size = v);
      }
    };

    let mut wnd = default_mock_window(w);
    // init size
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(20., 20.)));

    sleep(Duration::from_millis(10));

    // keep same
    wnd.run_futures();
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(20., 20.)));

    // trigger timeout
    wake_timeout_futures();
    wnd.run_futures();
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(10., 10.)));
  }

  fn env(times: usize) -> (Window, Rc<RefCell<usize>>) {
    let size = Size::new(400., 400.);
    let count = Rc::new(RefCell::new(0));
    let c_count = count.clone();
    let w = widget! {
      MockBox {
        size,
        on_x_times_tap: (times, move |_| *c_count.borrow_mut() += 1)
      }
    };
    let mut wnd = mock_window(w, size, <_>::default());
    wnd.draw_frame();

    (wnd, count)
  }

  fn run_until(local_pool: &mut LocalPool, cond: impl Fn() -> bool) {
    loop {
      wake_timeout_futures();
      local_pool.run_until_stalled();
      if (cond)() {
        break;
      }
      sleep(Duration::from_millis(8));
    }
  }

  pub fn test_double_tap() {
    let (mut wnd, count) = env(2);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(8)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        #[allow(deprecated)]
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    run_until(&mut local_pool, || *is_complete2.borrow());
    assert_eq!(*count.borrow(), 2);

    let (mut wnd, count) = env(2);
    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(251), local_pool.spawner())
      .take(8)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        #[allow(deprecated)]
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    run_until(&mut local_pool, || *is_complete2.borrow());
    assert_eq!(*count.borrow(), 0);
  }

  pub fn test_tripe_tap() {
    let (mut wnd, count) = env(3);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(12)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        #[allow(deprecated)]
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    run_until(&mut local_pool, || *is_complete2.borrow());

    assert_eq!(*count.borrow(), 2);
  }
}

fn main() {
  use colored::Colorize;
  let _ = NEW_TIMER_FN.set(new_timer);
  ribir_core::test::unit_test_describe! {
    run_unit_test(test_single_thread::test_widget_with_timer);
    run_unit_test(test_single_thread::test_double_tap);
    run_unit_test(test_single_thread::test_tripe_tap);
  }
}
