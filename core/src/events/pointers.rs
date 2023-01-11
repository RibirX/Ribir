use super::*;
use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

use std::{
  cell::RefCell,
  rc::Rc,
  time::{Duration, Instant},
};

mod from_mouse;
#[derive(Debug, Clone)]
pub struct PointerId(usize);

/// The pointer is a hardware-agnostic device that can target a specific set of
/// screen coordinates. Having a single event model for pointers can simplify
/// creating Web sites and applications and provide a good user experience
/// regardless of the user's hardware. However, for scenarios when
/// device-specific handling is desired, pointer events defines a pointerType
/// property to inspect the device type which produced the event.
/// Reference: <https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events#term_pointer_event>
#[derive(Debug, Clone)]
pub struct PointerEvent {
  /// A unique identifier for the pointer causing the event.
  pub id: PointerId,
  /// The width (magnitude on the X axis), in pixels, of the contact geometry of
  /// the pointer.
  pub width: f32,
  /// the height (magnitude on the Y axis), in pixels, of the contact geometry
  /// of the pointer.
  pub height: f32,
  /// the normalized pressure of the pointer input in the range of 0 to 1, where
  /// 0 and 1 represent the minimum and maximum pressure the hardware is capable
  /// of detecting, respectively. tangentialPressure
  /// The normalized tangential pressure of the pointer input (also known as
  /// barrel pressure or cylinder stress) in the range -1 to 1, where 0 is the
  /// neutral position of the control.
  pub pressure: f32,
  /// The plane angle (in degrees, in the range of -90 to 90) between the Y–Z
  /// plane and the plane containing both the pointer (e.g. pen stylus) axis and
  /// the Y axis.
  pub tilt_x: f32,
  /// The plane angle (in degrees, in the range of -90 to 90) between the X–Z
  /// plane and the plane containing both the pointer (e.g. pen stylus) axis and
  /// the X axis.
  pub tilt_y: f32,
  /// The clockwise rotation of the pointer (e.g. pen stylus) around its major
  /// axis in degrees, with a value in the range 0 to 359.
  pub twist: f32,
  ///  Indicates the device type that caused the event (mouse, pen, touch, etc.)
  pub point_type: PointerType,
  /// Indicates if the pointer represents the primary pointer of this pointer
  /// type.
  pub is_primary: bool,

  pub common: EventCommon,
}

bitflags! {
  #[derive(Default)]
  pub struct MouseButtons: u8 {
    /// Primary button (usually the left button)
    const PRIMARY = 0b0000_0001;
    /// Secondary button (usually the right button)
    const SECONDARY = 0b0000_0010;
    /// Auxiliary button (usually the mouse wheel button or middle button)
    const AUXILIARY = 0b0000_0100;
    /// 4th button (typically the "Browser Back" button)
    const FOURTH = 0b0000_1000;
    /// 5th button (typically the "Browser Forward" button)
    const FIFTH = 0b0001_0000;
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerType {
  /// The event was generated by a mouse device.
  Mouse,
  /// The event was generated by a pen or stylus device.
  Pen,
  /// The event was generated by a touch, such as a finger.
  Touch,
}

impl std::borrow::Borrow<EventCommon> for PointerEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for PointerEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for PointerEvent {
  type Target = EventCommon;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for PointerEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

type PointerCallback = Rc<RefCell<Box<dyn FnMut(&mut PointerEvent)>>>;
macro_rules! impl_pointer_listener {
  ($name: ident, $field: ident, $event_ty: ty) => {
    declare_builtin_event_field!($name, $field, $event_ty);
    impl ComposeChild for $name {
      type Child = Widget;
      #[inline]
      fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
        compose_child_as_data_widget(child, this)
      }
    }

    impl Query for $name {
      impl_query_self_only!();
    }
    impl_event_stream_dispatch!($name, $field, $event_ty);
  };
}

#[derive(Declare)]
pub struct PointerDownListener {
  #[declare(builtin, convert=custom)]
  pointer_down: PointerCallback,
  #[declare(skip)]
  pub pointer_down_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}

impl_pointer_listener!(PointerDownListener, pointer_down, PointerEvent);

#[derive(Declare)]
pub struct PointerUpListener {
  #[declare(builtin, convert=custom)]
  pointer_up: PointerCallback,
  #[declare(skip)]
  pub pointer_up_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(PointerUpListener, pointer_up, PointerEvent);

#[derive(Declare)]
pub struct PointerMoveListener {
  #[declare(builtin, convert=custom)]
  pointer_move: PointerCallback,
  #[declare(skip)]
  pub pointer_move_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(PointerMoveListener, pointer_move, PointerEvent);

#[derive(Declare)]
pub struct TapListener {
  #[declare(builtin, convert=custom)]
  tap: PointerCallback,
  #[declare(skip)]
  pub tap_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(TapListener, tap, PointerEvent);

#[derive(Declare)]
pub struct PointerCancelListener {
  #[declare(builtin, convert=custom)]
  pointer_cancel: PointerCallback,
  #[declare(skip)]
  pub pointer_cancel_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(PointerCancelListener, pointer_cancel, PointerEvent);

#[derive(Declare)]
pub struct PointerEnterListener {
  #[declare(builtin, convert=custom)]
  pointer_enter: PointerCallback,
  #[declare(skip)]
  pub pointer_enter_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(PointerEnterListener, pointer_enter, PointerEvent);

#[derive(Declare)]
pub struct PointerLeaveListener {
  #[declare(builtin, convert=custom)]
  pointer_leave: PointerCallback,
  #[declare(skip)]
  pub pointer_leave_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl_pointer_listener!(PointerLeaveListener, pointer_leave, PointerEvent);

#[derive(Declare)]
pub struct XTimesTapListener {
  #[declare(convert=custom, builtin)]
  pub x_times_tap: (u8, PointerCallback),
}

impl ComposeChild for XTimesTapListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget_maybe_states! {
      maybe_states { this }
      DynWidget {
        dyns: child,
        tap: {
          const DUR: Duration = Duration::from_millis(250);
          #[derive(Clone)]
          struct TapInfo {
            first_tap_stamp: Instant,
            tap_times: u8,
            pointer_type: PointerType,
            mouse_btns: MouseButtons,
          }
          let mut tap_info: Option<TapInfo> = None;
          move |e| {
            let (times, ref handler) = this.x_times_tap;
            match &mut tap_info {
              Some(info)
                if info.pointer_type == e.point_type
                  && info.mouse_btns == e.mouse_buttons()
                  && info.tap_times < times
                  && info.first_tap_stamp.elapsed() < DUR =>
              {
                info.tap_times += 1;
              }
              _ => {
                tap_info = Some(TapInfo {
                  first_tap_stamp: Instant::now(),
                  tap_times: 1,
                  pointer_type: e.point_type.clone(),
                  mouse_btns: e.mouse_buttons(),
                })
              }
            };

            let info = tap_info.as_mut().unwrap();
            if info.tap_times == times {
              info.tap_times = 0;
              (handler.borrow_mut())(e)
            }
          }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct DoubleTapListener {
  #[declare(builtin, convert=custom)]
  pub double_tap: PointerCallback,
  #[declare(skip)]
  pub double_tap_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}

#[derive(Declare)]
pub struct TripleTapListener {
  #[declare(convert=custom, builtin)]
  pub tripe_tap: PointerCallback,
  #[declare(skip)]
  pub tripe_tap_stream: LocalSubject<'static, Rc<RefCell<PointerEvent>>, ()>,
}
impl ComposeChild for DoubleTapListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      XTimesTapListener {
        x_times_tap: (2, move |e| this.double_tap_stream.next(Rc::new(RefCell::new(e.clone())))),
        DynWidget { dyns: child }
      }
    }
  }
}

impl ComposeChild for TripleTapListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      XTimesTapListener {
        x_times_tap: (3, move |e|  this.tripe_tap_stream.next(Rc::new(RefCell::new(e.clone())))),
        DynWidget { dyns: child }
      }
    }
  }
}

impl XTimesTapListenerDeclarer {
  #[inline]
  pub fn x_times_tap(mut self, f: (u8, impl FnMut(&mut PointerEvent) + 'static)) -> Self {
    self.x_times_tap = Some((f.0, Rc::new(RefCell::new(Box::new(f.1)))));
    self
  }
}

impl XTimesTapListener {
  #[inline]
  pub fn set_declare_x_times_tap(
    &mut self,
    f: (u8, impl for<'r> FnMut(&'r mut PointerEvent) + 'static),
  ) {
    self.x_times_tap = (f.0, Rc::new(RefCell::new(Box::new(f.1))));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::MockBox;
  use futures::executor::LocalPool;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton, WindowEvent};

  fn env(times: u8) -> (Window, Rc<RefCell<usize>>) {
    let size = Size::new(400., 400.);
    let count = Rc::new(RefCell::new(0));
    let c_count = count.clone();
    let w = widget! {
      MockBox {
        size,
        x_times_tap: (times,  move |_| *c_count.borrow_mut() += 1),
      }
    };
    let mut wnd = Window::default_mock(w, Some(size));
    wnd.draw_frame();

    (wnd, count)
  }

  #[test]
  fn double_tap() {
    let (mut wnd, count) = env(2);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(8)
      .subscribe(move |i| {
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

    local_pool.run();

    assert_eq!(*count.borrow(), 2);

    let (mut wnd, count) = env(2);
    observable::interval(Duration::from_millis(251), local_pool.spawner())
      .take(8)
      .subscribe(move |i| {
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

    local_pool.run();
    assert_eq!(*count.borrow(), 0);
  }

  #[test]
  fn tripe_tap() {
    let (mut wnd, count) = env(3);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(12)
      .subscribe(move |i| {
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

    local_pool.run();

    assert_eq!(*count.borrow(), 2);
  }
}
