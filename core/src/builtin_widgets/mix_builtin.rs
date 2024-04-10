use std::{cell::Cell, convert::Infallible};

use rxrust::prelude::*;

use self::focus_mgr::FocusType;
use crate::prelude::*;

const MULTI_TAP_DURATION: Duration = Duration::from_millis(250);

bitflags! {
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub struct BuiltinFlags: u64 {
    // Listener flags, the flags are used to indicate what
    // kind of events the widget are listening to.
    const Lifecycle = 1 << 0;
    /// Pointer listener flag, hint the widget is listening to pointer events
    const Pointer = 1 << 1;
    /// Wheel listener flag, hint the widget is listening to wheel events
    const Wheel = 1 << 2;
    /// Keyboard listener flag, hint the widget is listening to keyboard events
    const KeyBoard = 1 << 3 | Self::Focus.bits();
    /// Whether the widget is a focus node also hint the widget
    /// is listening to focus/blur events
    const Focus = 1 << 4;
    /// Bubble focus event listener flag, hint the widget is listening to
    /// FocusIn/FocusOut and their capture events
    const FocusInOut = 1 << 5;

    const AllListeners = Self::Lifecycle.bits()
      | Self::Pointer.bits()
      | Self::Wheel.bits()
      | Self::KeyBoard.bits()
      | Self::Focus.bits()
      | Self::FocusInOut.bits();
    // listener end

    const AutoFocus = 1 << 47;
    // 16 bits keep for tab index
  }
}

pub type EventSubject = MutRefItemSubject<'static, Event, Infallible>;

#[derive(Default, Query)]
pub struct MixBuiltin {
  flags: Cell<BuiltinFlags>,
  subject: EventSubject,
}

impl Declare for MixBuiltin {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

macro_rules! event_map_filter {
  ($event_name:ident, $event_ty:ident) => {
    (|e| match e {
      Event::$event_name(e) => Some(e),
      _ => None,
    }) as fn(&mut Event) -> Option<&mut $event_ty>
  };
}

macro_rules! impl_event_callback {
  ($this:ident, $listen_type:ident, $event_name:ident, $event_ty:ident, $handler:ident) => {{
    $this.flag_mark(BuiltinFlags::$listen_type);
    let _ = $this
      .subject()
      .filter_map(event_map_filter!($event_name, $event_ty))
      .subscribe($handler);

    $this
  }};
}

impl MixBuiltin {
  #[inline]
  pub fn contain_flag(&self, t: BuiltinFlags) -> bool { self.flags.get().contains(t) }

  pub fn flag_mark(&self, t: BuiltinFlags) {
    let t = self.flags.get() | t;
    self.flags.set(t)
  }

  pub fn dispatch(&self, event: &mut Event) { self.subject.clone().next(event) }

  pub fn subject(&self) -> EventSubject { self.subject.clone() }

  /// Listen to all events
  pub fn on_event(&self, handler: impl FnMut(&mut Event) + 'static) -> &Self {
    self.flag_mark(BuiltinFlags::AllListeners);
    let _ = self.subject().subscribe(handler);
    self
  }

  pub fn on_mounted(&self, handler: impl FnOnce(&mut LifecycleEvent) + 'static) -> &Self {
    self.flag_mark(BuiltinFlags::Lifecycle);
    let _ = self
      .subject()
      .filter_map(event_map_filter!(Mounted, LifecycleEvent))
      .take(1)
      .subscribe(life_fn_once_to_fn_mut(handler));

    self
  }

  pub fn on_performed_layout(&self, handler: impl FnMut(&mut LifecycleEvent) + 'static) -> &Self {
    impl_event_callback!(self, Lifecycle, PerformedLayout, LifecycleEvent, handler)
  }

  pub fn on_disposed(&self, handler: impl FnOnce(&mut LifecycleEvent) + 'static) -> &Self {
    self.flag_mark(BuiltinFlags::Lifecycle);
    let _ = self
      .subject()
      .filter_map(event_map_filter!(Disposed, LifecycleEvent))
      .take(1)
      .subscribe(life_fn_once_to_fn_mut(handler));

    self
  }

  pub fn on_pointer_down(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerDown, PointerEvent, handler)
  }

  pub fn on_pointer_down_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerDownCapture, PointerEvent, handler)
  }

  pub fn on_pointer_up(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerUp, PointerEvent, handler)
  }

  pub fn on_pointer_up_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerUpCapture, PointerEvent, handler)
  }

  pub fn on_pointer_move(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerMove, PointerEvent, handler)
  }

  pub fn on_pointer_move_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerMoveCapture, PointerEvent, handler)
  }

  pub fn on_pointer_cancel(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerCancel, PointerEvent, handler)
  }

  pub fn on_pointer_enter(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerEnter, PointerEvent, handler)
  }

  pub fn on_pointer_leave(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, PointerLeave, PointerEvent, handler)
  }

  pub fn on_tap(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, Tap, PointerEvent, handler)
  }

  pub fn on_tap_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    impl_event_callback!(self, Pointer, TapCapture, PointerEvent, handler)
  }

  pub fn on_double_tap(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    self.on_x_times_tap((2, handler))
  }

  pub fn on_double_tap_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    self.on_x_times_tap_capture((2, handler))
  }

  pub fn on_triple_tap(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    self.on_x_times_tap((3, handler))
  }

  pub fn on_triple_tap_capture(&self, handler: impl FnMut(&mut PointerEvent) + 'static) -> &Self {
    self.on_x_times_tap_capture((3, handler))
  }

  pub fn on_x_times_tap(
    &self, (times, handler): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> &Self {
    self.on_x_times_tap_impl(times, MULTI_TAP_DURATION, false, handler)
  }

  pub fn on_x_times_tap_capture(
    &self, (times, handler): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> &Self {
    self.on_x_times_tap_impl(times, MULTI_TAP_DURATION, true, handler)
  }

  pub fn on_wheel(&self, handler: impl FnMut(&mut WheelEvent) + 'static) -> &Self {
    impl_event_callback!(self, Wheel, Wheel, WheelEvent, handler)
  }

  pub fn on_wheel_capture(&self, handler: impl FnMut(&mut WheelEvent) + 'static) -> &Self {
    impl_event_callback!(self, Wheel, WheelCapture, WheelEvent, handler)
  }

  fn on_x_times_tap_impl(
    &self, times: usize, dur: Duration, capture: bool,
    handler: impl FnMut(&mut PointerEvent) + 'static,
  ) -> &Self {
    self.flag_mark(BuiltinFlags::Pointer);
    self
      .subject()
      .filter_map(x_times_tap_map_filter(times, dur, capture))
      .subscribe(handler);
    self
  }

  pub fn on_ime_pre_edit(&self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, ImePreEdit, ImePreEditEvent, f)
  }

  pub fn on_ime_pre_edit_capture(&self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, ImePreEditCapture, ImePreEditEvent, f)
  }

  pub fn on_chars(&self, f: impl FnMut(&mut CharsEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, Chars, CharsEvent, f)
  }

  pub fn on_chars_capture(&self, f: impl FnMut(&mut CharsEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, CharsCapture, CharsEvent, f)
  }

  pub fn on_key_down(&self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, KeyDown, KeyboardEvent, f)
  }

  pub fn on_key_down_capture(&self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, KeyDownCapture, KeyboardEvent, f)
  }

  pub fn on_key_up(&self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, KeyUp, KeyboardEvent, f)
  }

  pub fn on_key_up_capture(&self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &Self {
    impl_event_callback!(self, KeyBoard, KeyUpCapture, KeyboardEvent, f)
  }

  pub fn on_focus(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, Focus, Focus, FocusEvent, f)
  }

  pub fn on_blur(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, Focus, Blur, FocusEvent, f)
  }

  pub fn on_focus_in(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, FocusInOut, FocusIn, FocusEvent, f)
  }

  pub fn on_focus_in_capture(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, FocusInOut, FocusInCapture, FocusEvent, f)
  }

  pub fn on_focus_out(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, FocusInOut, FocusOut, FocusEvent, f)
  }

  pub fn on_focus_out_capture(&self, f: impl FnMut(&mut FocusEvent) + 'static) -> &Self {
    impl_event_callback!(self, FocusInOut, FocusOutCapture, FocusEvent, f)
  }

  /// Indicates that `widget` can be focused, and where it participates in
  /// sequential keyboard navigation (usually with the Tab key).
  pub fn is_focus_node(&self) -> bool { self.flags.get().contains(BuiltinFlags::Focus) }

  pub fn get_tab_index(&self) -> i16 { (self.flags.get().bits() >> 48) as i16 }

  pub fn set_tab_index(&self, tab_idx: i16) -> &Self {
    self.flag_mark(BuiltinFlags::Focus);
    let flags = self.flags.get().bits() | ((tab_idx as u64) << 48);
    self
      .flags
      .set(BuiltinFlags::from_bits_retain(flags));
    self
  }

  pub fn is_auto_focus(&self) -> bool { self.flags.get().contains(BuiltinFlags::AutoFocus) }

  pub fn set_auto_focus(&self, v: bool) -> &Self {
    if v {
      self.flag_mark(BuiltinFlags::AutoFocus | BuiltinFlags::Focus);
    } else {
      let mut flag = self.flags.get();
      flag.remove(BuiltinFlags::AutoFocus);
      self.flags.set(flag);
    }
    self
  }

  fn merge(&self, other: Self) {
    let tab_index = self.get_tab_index();
    let other_tab_index = other.get_tab_index();
    self
      .flags
      .set(self.flags.get() | other.flags.get());
    if other_tab_index != 0 {
      self.set_tab_index(other_tab_index);
    } else if tab_index != 0 {
      self.set_tab_index(tab_index);
    }

    let other_subject = other.subject();
    fn subscribe_fn(subject: EventSubject) -> impl FnMut(&mut Event) {
      move |e: &mut Event| {
        subject.clone().next(e);
      }
    }
    self
      .subject()
      .subscribe(subscribe_fn(other_subject));
  }

  fn callbacks_for_focus_node(&self) {
    self
      .on_mounted(move |e| {
        e.query_type(|mix: &MixBuiltin| {
          let auto_focus = mix.is_auto_focus();
          e.window()
            .add_focus_node(e.id, auto_focus, FocusType::Node)
        });
      })
      .on_disposed(|e| {
        e.window()
          .remove_focus_node(e.id, FocusType::Node)
      });
  }
}

fn life_fn_once_to_fn_mut(
  handler: impl FnOnce(&mut LifecycleEvent),
) -> impl FnMut(&mut LifecycleEvent) {
  let mut handler = Some(handler);
  move |e| {
    if let Some(h) = handler.take() {
      h(e);
    }
  }
}

impl ComposeChild for MixBuiltin {
  type Child = Widget;
  #[inline]
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    move |ctx: &BuildCtx| match this.try_into_value() {
      Ok(this) => {
        let mut this = Some(this);
        child
          .id()
          .assert_get(&ctx.tree.borrow().arena)
          .query_most_outside(|m: &MixBuiltin| {
            let this = this.take().unwrap();
            if !m.contain_flag(BuiltinFlags::Focus) && this.contain_flag(BuiltinFlags::Focus) {
              this.callbacks_for_focus_node();
            }
            m.merge(this)
          });
        if let Some(this) = this {
          if this.contain_flag(BuiltinFlags::Focus) {
            this.callbacks_for_focus_node();
          }
          child.attach_data(this, ctx)
        } else {
          child
        }
      }
      Err(this) => {
        if this.read().contain_flag(BuiltinFlags::Focus) {
          this.read().callbacks_for_focus_node();
        }
        child.attach_data(this, ctx)
      }
    }
  }
}

fn x_times_tap_map_filter(
  x: usize, dur: Duration, capture: bool,
) -> impl FnMut(&mut Event) -> Option<&mut PointerEvent> {
  assert!(x > 0);
  struct TapInfo {
    pointer_id: PointerId,
    stamps: Vec<Instant>,
  }

  let mut type_info: Option<TapInfo> = None;
  move |e: &mut Event| {
    let e = match e {
      Event::Tap(e) if !capture => e,
      Event::TapCapture(e) if capture => e,
      _ => return None,
    };
    let now = Instant::now();
    match &mut type_info {
      Some(info) if info.pointer_id == e.id => {
        if info.stamps.len() + 1 == x {
          if now.duration_since(info.stamps[0]) <= dur {
            // emit x-tap event and reset the tap info
            type_info = None;
            Some(e)
          } else {
            // remove the expired tap
            info.stamps.remove(0);
            info.stamps.push(now);
            None
          }
        } else {
          info.stamps.push(now);
          None
        }
      }
      _ => {
        type_info = Some(TapInfo { pointer_id: e.id, stamps: vec![now] });
        None
      }
    }
  }
}
