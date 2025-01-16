use std::{cell::RefCell, convert::Infallible};

use rxrust::prelude::*;

use self::focus_mgr::FocusType;
use crate::prelude::*;

const MULTI_TAP_DURATION: Duration = Duration::from_millis(250);

bitflags! {
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub struct MixFlags: u64 {
    // Listener flags, the flags are used to indicate what
    // kind of events the widget are listening to.
    const Lifecycle = 1 << 0;
    #[doc="Pointer listener flag, hint the widget is listening to pointer events"]
    const Pointer = 1 << 1;
    #[doc="Wheel listener flag, hint the widget is listening to wheel events"]
    const Wheel = 1 << 2;
    #[doc="Keyboard listener flag, hint the widget is listening to keyboard events"]
    const KeyBoard = 1 << 3 | Self::Focus.bits();
    #[doc="Whether the widget is a focus node also hint the widget \
    is listening to focus/blur events"]
    const Focus = 1 << 4;
    #[doc="Bubble focus event listener flag, hint the widget is listening to \
     FocusIn/FocusOut and their capture events"]
    const FocusInOut = 1 << 5;

    const AllListeners = Self::Lifecycle.bits()
      | Self::Pointer.bits()
      | Self::Wheel.bits()
      | Self::KeyBoard.bits()
      | Self::Focus.bits()
      | Self::FocusInOut.bits();
    // listener end

    #[doc="Indicates whether this widget is tracing its focus status."]
    const TraceFocus = 1 << 16;
    #[doc="Indicates whether the focus is on this widget (including its descendants)."]
    const Focused = 1 << 17;
    #[doc="Indicates whether this widget is tracing the hover status."]
    const TraceHover = 1 << 18;
    #[doc="Indicates whether the mouse is hover on this widget (including its descendants)."]
    const Hovered = 1 << 19;
    #[doc="Indicates whether this widget is tracing the pressed status of pointer."]
    const TracePointerPressed = 1 << 20;
    #[doc="Indicates whether the pointer is pressed on this widget."]
    const PointerPressed = 1 << 21;

    #[doc="Indicates whether this widget has auto-focus functionality."]
    const AutoFocus = 1 << 47;
    // The last 16 bits keep for tab index
  }
}

pub type EventSubject = MutRefItemSubject<'static, Event, Infallible>;

pub struct MixBuiltin {
  flags: State<MixFlags>,
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
    $this.silent_mark(MixFlags::$listen_type);
    let _ = $this
      .subject()
      .filter_map(event_map_filter!($event_name, $event_ty))
      .subscribe($handler);

    $this
  }};
}

impl MixFlags {
  /// Indicates whether the focus is on this widget (including its children).
  ///
  /// By default, the focus status is not traced. You need to call
  /// `MixBuiltin::trace_focus` to start recording the focus status of
  /// this widget. If you do not call `MixBuiltin::trace_focus` when
  /// this widget is created, this method will always return `false`, even if it
  /// has focus.
  pub fn has_focus(&self) -> bool { self.contains(MixFlags::Focused) }

  /// Indicates whether the mouse is hovering over this widget (including its
  /// children).
  ///
  /// By default, the hover status is not traced. You need to call
  /// `MixBuiltin::trace_hover` to start tracking the focus status of
  /// this widget. If you do not call `MixBuiltin::trace_hover` when this
  /// widget is created, this method will always return false, even if the mouse
  /// is hovering over it.
  pub fn is_hover(&self) -> bool { self.contains(MixFlags::Hovered) }

  /// Indicates whether the the pointer is pressed on this widget.
  ///
  /// By default, the pressed status is not traced. You need to call
  /// `MixBuiltin::trace_pointer_pressed` to start tracking the focus status of
  /// this widget. If you do not call `MixBuiltin::trace_pointer_pressed` when
  /// this widget is created, this method will always return false, even if
  /// the mouse is hovering over it.
  pub fn is_pointer_pressed(&self) -> bool { self.contains(MixFlags::PointerPressed) }

  pub fn is_auto_focus(&self) -> bool { self.contains(MixFlags::AutoFocus) }

  pub fn set_auto_focus(&mut self, v: bool) {
    if v {
      self.insert(MixFlags::AutoFocus | MixFlags::Focus);
    } else {
      self.remove(MixFlags::AutoFocus);
    }
  }

  pub fn tab_index(&self) -> Option<i16> {
    self
      .contains(MixFlags::Focus)
      .then(|| (self.bits() >> 48) as i16)
  }

  pub fn set_tab_index(&mut self, tab_idx: i16) {
    self.insert(MixFlags::Focus);
    let flags = self.bits() | ((tab_idx as u64) << 48);
    *self = MixFlags::from_bits_retain(flags);
  }
}

impl MixBuiltin {
  pub fn mix_flags(&self) -> &State<MixFlags> { &self.flags }

  pub fn dispatch(&self, event: &mut Event) { self.subject.clone().next(event) }

  /// Listen to all events
  pub fn on_event(&self, handler: impl FnMut(&mut Event) + 'static) -> &Self {
    self.silent_mark(MixFlags::AllListeners);
    let _ = self.subject().subscribe(handler);
    self
  }

  pub fn on_mounted(&self, handler: impl FnOnce(&mut LifecycleEvent) + 'static) -> &Self {
    self.silent_mark(MixFlags::Lifecycle);
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
    self.silent_mark(MixFlags::Lifecycle);
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
    self.silent_mark(MixFlags::Pointer);
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

  /// Begin tracing the focus status of this widget.
  pub fn trace_focus(&self) {
    if !self.contain_flag(MixFlags::TraceFocus) {
      self.silent_mark(MixFlags::TraceFocus);
      let flags = self.flags.clone_writer();
      self.on_focus_in(move |_| flags.write().insert(MixFlags::Focused));
      let flags = self.flags.clone_writer();
      self.on_focus_out(move |_| flags.write().remove(MixFlags::Focused));
    }
  }

  /// Begin tracing the hover status of this widget.
  pub fn trace_hover(&self) {
    if !self.contain_flag(MixFlags::TraceHover) {
      self.silent_mark(MixFlags::TraceHover);
      let flags = self.flags.clone_writer();
      self.on_pointer_enter(move |_| flags.write().insert(MixFlags::Hovered));
      let flags = self.flags.clone_writer();
      self.on_pointer_leave(move |_| flags.write().remove(MixFlags::Hovered));
    }
  }

  /// Begin tracing if the pointer pressed on this widget
  pub fn trace_pointer_pressed(&self) {
    if !self.contain_flag(MixFlags::TracePointerPressed) {
      self.silent_mark(MixFlags::TracePointerPressed);
      let flags = self.flags.clone_writer();
      self.on_pointer_down(move |_| flags.write().insert(MixFlags::PointerPressed));
      let flags = self.flags.clone_writer();
      self.on_pointer_up(move |_| flags.write().remove(MixFlags::PointerPressed));
    }
  }

  fn subject(&self) -> EventSubject { self.subject.clone() }

  pub(crate) fn contain_flag(&self, t: MixFlags) -> bool { self.flags.read().contains(t) }

  fn silent_mark(&self, t: MixFlags) {
    let mut w = self.flags.write();
    w.insert(t);
    w.forget_modifies();
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

fn callbacks_for_focus_node(child: Widget) -> Widget {
  fn_widget! {
  let guard = Sc::new(RefCell::new(None));
  let guard2 = guard.clone();
  let mut child = FatObj::new(child);
  @$child {
    on_mounted: move |e| {
      let mut all_mix = e.query_all_iter::<MixBuiltin>().peekable();
      if all_mix.peek().is_some() {
        let auto_focus = all_mix.any(|mix| mix.flags.read().is_auto_focus());
        let track_id = $child.track_id().watcher();
        let wnd = e.window();
        let init_id = e.id();
        wnd.add_focus_node(init_id, auto_focus, FocusType::Node);
        *guard2.borrow_mut() = Some(
          watch!(*$track_id)
            .merge(observable::of(Some(init_id)))
            .distinct_until_changed()
            .pairwise()
            .subscribe(move |(old, new)| {
              if let Some(wid) = old {
                wnd.remove_focus_node(wid, FocusType::Node);
              }
              if let Some(wid) = new {
                wnd.add_focus_node(wid, auto_focus, FocusType::Node);
              }
            }).unsubscribe_when_dropped());
        }
      },
      on_disposed: move |e| {
        guard.borrow_mut().take();
        e.window().remove_focus_node(e.id(), FocusType::Node);
      }
    }
  }
  .into_widget()
}

impl<'c> ComposeChild<'c> for MixBuiltin {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, mut child: Self::Child) -> Widget<'c> {
    let mix = this.read();
    if mix.contain_flag(MixFlags::Focus) {
      child = callbacks_for_focus_node(child);
    }
    if !mix.subject.is_empty() {
      let subject = mix.subject.clone();
      mix.on_disposed(move |_| {
        let _ = AppCtx::spawn_local(async move { subject.unsubscribe() });
      });
    }
    drop(mix);
    child.try_unwrap_state_and_attach(this)
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

impl Default for MixBuiltin {
  fn default() -> Self {
    Self { flags: State::value(MixFlags::default()), subject: Default::default() }
  }
}

impl Clone for MixBuiltin {
  fn clone(&self) -> Self {
    let flags = self.flags.clone_writer();
    Self { flags, subject: self.subject.clone() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn mix_should_not_merge() {
    reset_test_env!();

    let (trigger, w_trigger) = split_value(0);
    let (outer_layout, w_outer_layout) = split_value(0);
    let mix_keep = fn_widget! {
      let pipe_w = FatObj::new( pipe! {
        $trigger;
        @Void { on_performed_layout: move |_| {} }
      });

      @ $pipe_w {
        on_performed_layout: move |_| *$w_outer_layout.write() +=1 ,
      }
    };

    let mut wnd = TestWindow::new(mix_keep);
    wnd.draw_frame();
    assert_eq!(*outer_layout.read(), 1);

    *w_trigger.write() = 1;

    wnd.draw_frame();
    assert_eq!(*outer_layout.read(), 2);
  }
}
