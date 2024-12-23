use ribir::core::timer::Timer;
use rxrust::scheduler::NEW_TIMER_FN;

mod test_single_thread {
  use std::{cell::RefCell, rc::Rc, thread::sleep};

  use ribir_core::{prelude::*, reset_test_env, test_helper::*};
  use winit::event::{DeviceId, ElementState, MouseButton};

  pub fn test_widget_with_timer() {
    let w = fn_widget! {
      let c = @MockBox { size: Size::new(20., 20.) };
      observable::of(Size::new(10., 10.))
        .delay(Duration::from_millis(10), AppCtx::scheduler())
        .subscribe(move |v| $c.write().size = v);
      c
    };

    let mut wnd = TestWindow::new(w);
    // init size
    wnd.draw_frame();
    wnd.assert_root_size((20., 20.).into());

    // keep same
    AppCtx::run_until_stalled();
    wnd.draw_frame();
    wnd.assert_root_size((20., 20.).into());

    sleep(Duration::from_millis(10));

    // trigger timeout
    super::Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    wnd.draw_frame();
    wnd.assert_root_size((10., 10.).into());
  }

  fn env(times: usize) -> (TestWindow, Watcher<Reader<i32>>) {
    let size = Size::new(400., 400.);
    let (count, w_count) = split_value(0);

    let w = fn_widget! {
      @MockBox {
        size,
        on_x_times_tap: (times, move |_| *$w_count.write() +=1 )
      }
    };
    let mut wnd = TestWindow::new_with_size(w, size);
    wnd.draw_frame();

    (wnd, count)
  }

  fn run_until(wnd: &TestWindow, cond: impl Fn() -> bool) {
    loop {
      Timer::wake_timeout_futures();
      AppCtx::run_until_stalled();
      wnd.run_frame_tasks();

      if (cond)() {
        break;
      }
      sleep(Duration::from_millis(1));
    }
  }

  pub fn test_double_tap() {
    reset_test_env!();
    let (wnd, count) = env(2);
    let c_wnd = wnd.clone();

    let device_id = unsafe { DeviceId::dummy() };
    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(10), AppCtx::scheduler())
      .take(8)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        let state = if i % 2 == 0 { ElementState::Pressed } else { ElementState::Released };
        c_wnd.process_mouse_input(device_id, state, MouseButton::Left);
      });

    run_until(&wnd, || *is_complete2.borrow());
    assert_eq!(*count.read(), 2);

    let (wnd, count) = env(2);
    let c_wnd = wnd.clone();

    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(251), AppCtx::scheduler())
      .take(8)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        c_wnd.process_mouse_input(
          device_id,
          if i % 2 == 0 { ElementState::Pressed } else { ElementState::Released },
          MouseButton::Left,
        );
      });

    run_until(&wnd, || *is_complete2.borrow());
    assert_eq!(*count.read(), 0);
  }

  pub fn test_tripe_tap() {
    reset_test_env!();
    let (wnd, count) = env(3);
    let c_wnd = wnd.clone();

    let device_id = unsafe { DeviceId::dummy() };
    let is_complete = Rc::new(RefCell::new(false));
    let is_complete2 = is_complete.clone();
    observable::interval(Duration::from_millis(10), AppCtx::scheduler())
      .take(12)
      .on_complete(move || {
        *is_complete.borrow_mut() = true;
      })
      .subscribe(move |i| {
        c_wnd.process_mouse_input(
          device_id,
          if i % 2 == 0 { ElementState::Pressed } else { ElementState::Released },
          MouseButton::Left,
        );
      });

    run_until(&wnd, || *is_complete2.borrow());

    assert_eq!(*count.read(), 2);
  }
}

fn main() {
  use colored::Colorize;
  use ribir_dev_helper::unit_test_describe;

  let _ = NEW_TIMER_FN.set(Timer::new_timer_future);
  unit_test_describe! {
    run_unit_test(test_single_thread::test_widget_with_timer);
    run_unit_test(test_single_thread::test_double_tap);
    run_unit_test(test_single_thread::test_tripe_tap);
  }
}
