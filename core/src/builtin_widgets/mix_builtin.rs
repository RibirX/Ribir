use std::{cell::RefCell, convert::Infallible};

use rxrust::prelude::*;

use self::focus_mgr::FocusType;
use crate::prelude::{window::WindowId, *};

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

    #[doc="Bubble custom event listener flag, hint the widget is listening to \
     custom events"]
    const Customs = 1 << 6;

    const AllListeners = Self::Lifecycle.bits()
      | Self::Pointer.bits()
      | Self::Wheel.bits()
      | Self::KeyBoard.bits()
      | Self::Focus.bits()
      | Self::FocusInOut.bits()
      | Self::Customs.bits();
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
    const AutoFocus = 1 << 22;

    // The last 32 bits keep to store data:
    // - 2 bits for focus reason(32..34)
    // - 16 bits for tab index(34..50)
    // - reserved
  }
}

/// Bits storing focus reason (2 bits)
const FOCUS_REASON_SHIFT: u64 = 32;
const FOCUS_REASON_MASK: u64 = 0b11 << FOCUS_REASON_SHIFT;
/// Bit range storing tab index (16 bits)
const TAB_IDX_SHIFT: u64 = 34;
const TAB_IDX_MASK: u64 = 0xFFFF << TAB_IDX_SHIFT;

pub type EventSubject = LocalSubjectMutRef<'static, Event, Infallible>;

pub struct MixBuiltin {
  flags: Stateful<MixFlags>,
  subject: EventSubject,
}

impl Declare for MixBuiltin {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

macro_rules! impl_event_callback {
  ($this:ident, $listen_type:ident, $event_name:ident, $event_ty:ty, $handler:ident) => {{
    $this.silent_mark(MixFlags::$listen_type);
    let mut handler = $handler;
    let _ = $this.subject().subscribe(move |e: &mut Event| {
      if let Event::$event_name(inner) = e {
        handler(inner);
      }
    });

    $this
  }};
}

impl MixFlags {
  /// Checks if the widget or any descendant currently has focus visibility.
  ///
  /// Focus tracking must be explicitly enabled during widget creation by
  /// calling `MixBuiltin::trace_focus`. Without initialization, this method
  /// will consistently return `false` regardless of actual focus state.
  #[inline]
  pub fn is_focused(&self) -> bool { self.contains(MixFlags::Focused) }

  /// Retrieves the reason for the most recent focus change event.
  ///
  /// When focused: indicates why focus was gained  
  /// When unfocused: indicates why focus was lost
  ///
  /// # Note
  /// Meaningful only when focus tracking is enabled via
  /// `MixBuiltin::trace_focus`. The return value becomes undefined if focus
  /// tracing wasn't initialized.
  pub fn focus_changed_reason(&self) -> FocusReason {
    let reason = (self.bits() & FOCUS_REASON_MASK) >> FOCUS_REASON_SHIFT;
    FocusReason::from_u8(reason as u8)
  }

  /// Determines if the widget or any descendant is currently hovered.
  ///
  /// Hover tracking requires explicit initialization via
  /// `MixBuiltin::trace_hover` during widget creation. Uninitialized hover
  /// state will always return `false`.
  #[inline]
  pub fn is_hovered(&self) -> bool { self.contains(MixFlags::Hovered) }

  /// Checks for active pointer press within widget boundaries.
  ///
  /// Requires prior initialization with `MixBuiltin::trace_pointer_pressed`.
  #[inline]
  pub fn is_pointer_pressed(&self) -> bool { self.contains(MixFlags::PointerPressed) }

  /// Indicates whether auto-focus is enabled for initial view activation.
  #[inline]
  pub fn auto_focus(&self) -> bool { self.contains(MixFlags::AutoFocus) }

  /// Configures auto-focus behavior with proper flag management.
  ///
  /// Enables/disables automatic focus acquisition during view activation,
  /// maintaining valid flag combinations.
  pub fn set_auto_focus(&mut self, enable: bool) {
    if enable {
      self.insert(MixFlags::AutoFocus | MixFlags::Focus);
    } else {
      self.remove(MixFlags::AutoFocus);
    }
  }

  /// Retrieves validated tab index for keyboard navigation.
  ///
  /// Returns `None` if tab navigation is disabled (Focus flag unset).
  /// The returned value is guaranteed to be within valid bounds.
  pub fn tab_index(&self) -> Option<i16> {
    self.contains(MixFlags::Focus).then(|| {
      let tab_idx = (self.bits() & TAB_IDX_MASK) >> TAB_IDX_SHIFT;
      tab_idx as i16
    })
  }

  /// Updates tab index with value sanitization and layout invalidation.
  ///
  /// Automatically enables focus tracking and clamps values to valid ranges.
  pub fn set_tab_index(&mut self, tab_idx: i16) {
    self.insert(MixFlags::Focus);
    let flags = (self.bits() & !TAB_IDX_MASK) | ((tab_idx as u64) << TAB_IDX_SHIFT);
    *self = MixFlags::from_bits_retain(flags);
  }

  /// (Internal) Updates focus reason flags while maintaining state integrity.
  fn set_focus_reason(&mut self, reason: FocusReason) {
    let flags = (self.bits() & !FOCUS_REASON_MASK) | ((reason as u64) << FOCUS_REASON_SHIFT);
    *self = MixFlags::from_bits_retain(flags);
  }
}

impl MixBuiltin {
  pub fn mix_flags(&self) -> &Stateful<MixFlags> { &self.flags }

  pub fn focus_handle(&self, wnd: WindowId, host: TrackId) -> FocusHandle {
    self.trace_focus();

    FocusHandle { flags: self.flags.clone_writer(), wnd, host }
  }

  pub fn dispatch(&self, event: &mut Event) { self.subject.clone().next(event) }

  /// Listen to all events
  pub fn on_event(&self, handler: impl FnMut(&mut Event) + 'static) -> &Self {
    self.silent_mark(MixFlags::AllListeners);
    let _ = self.subject().subscribe(handler);
    self
  }

  pub fn on_mounted(&self, handler: impl FnOnce(&mut LifecycleEvent) + 'static) -> &Self {
    self.silent_mark(MixFlags::Lifecycle);

    let mut handler = life_fn_once_to_fn_mut(handler);
    self
      .subject()
      .filter(|e| matches!(e, Event::Mounted(_)))
      .first()
      .subscribe(move |e: &mut Event| {
        if let Event::Mounted(inner) = e {
          handler(inner);
        }
      });

    self
  }

  pub fn on_performed_layout(&self, handler: impl FnMut(&mut LifecycleEvent) + 'static) -> &Self {
    impl_event_callback!(self, Lifecycle, PerformedLayout, LifecycleEvent, handler)
  }

  pub fn on_disposed(&self, handler: impl FnOnce(&mut LifecycleEvent) + 'static) -> &Self {
    self.silent_mark(MixFlags::Lifecycle);

    let mut handler = life_fn_once_to_fn_mut(handler);
    self
      .subject()
      .filter(|e| matches!(e, Event::Disposed(_)))
      .first()
      .subscribe(move |e: &mut Event| {
        if let Event::Disposed(inner) = e {
          handler(inner);
        }
      });

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
    let mut map_filter = x_times_tap_map_filter(times, dur, capture);
    let mut handler = handler;
    self.subject().subscribe(move |e: &mut Event| {
      if let Some(e) = map_filter(e) {
        handler(e);
      }
    });
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

  pub fn on_action(&self, handler: impl FnMut(&mut Event) + 'static) -> &Self {
    self.silent_mark(MixFlags::Pointer | MixFlags::KeyBoard);
    let mut handler = handler;

    let sub_action = self.subject().subscribe(move |e: &mut Event| {
      if let Event::Tap(_) = e {
        (handler)(e);
      } else if let Event::KeyDown(k) = e
        && matches!(k.key(), VirtualKey::Named(NamedKey::Enter))
        && !k.is_repeat()
      {
        (handler)(e);
      } else if let Event::KeyUp(k) = e
        && matches!(k.key(), VirtualKey::Named(NamedKey::Space))
      {
        (handler)(e);
      }
    });

    self.on_disposed(move |_| {
      sub_action.unsubscribe();
    });

    self
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

  pub fn on_custom<E: 'static, F: FnMut(&mut CustomEvent<E>) + 'static>(&self, mut f: F) -> &Self {
    let wrap_f = move |arg: &mut RawCustomEvent| {
      if let Some(e) = arg.downcast_mut::<E>() {
        f(e);
      }
    };
    impl_event_callback!(self, Customs, CustomEvent, RawCustomEvent, wrap_f)
  }

  pub fn on_raw_custom(&self, f: impl FnMut(&mut RawCustomEvent) + 'static) -> &Self {
    impl_event_callback!(self, Customs, CustomEvent, RawCustomEvent, f)
  }

  /// Begin tracing the focus status of this widget.
  /// Begin tracing the focus status of this widget.
  pub fn trace_focus(&self) {
    if !self.contain_flag(MixFlags::TraceFocus) {
      self.silent_mark(MixFlags::TraceFocus);
      let flags = self.flags.clone_writer();
      self.on_focus_in(move |e| {
        let mut flags = flags.write();
        flags.insert(MixFlags::Focused);
        flags.set_focus_reason(e.reason);
      });
      let flags = self.flags.clone_writer();
      self.on_focus_out(move |e| {
        let mut flags = flags.write();
        flags.set_focus_reason(e.reason);
        flags.remove(MixFlags::Focused)
      });
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
    let guard = Rc::new(RefCell::new(None));
    let guard2 = guard.clone();
    let mut child = FatObj::new(child);
    @(child) {
      on_mounted: move |e| {
        let mut all_mix = e.query_all_iter::<MixBuiltin>().peekable();
        if all_mix.peek().is_some() {
          let auto_focus = all_mix.any(|mix| mix.flags.read().auto_focus());
          *guard.borrow_mut() = Some(Window::add_focus_node(
            e.window(),
            $clone(child.track_id()),
            auto_focus, FocusType::Node
          ));
        }
      },
      on_disposed: move |_| { guard2.borrow_mut().take(); }
    }
  }
  .into_widget()
}

/// A handle to help for tracking focus status and control the focus for the
/// host widget.
pub struct FocusHandle {
  pub(super) flags: Stateful<MixFlags>,
  wnd: WindowId,
  host: TrackId,
}

impl FocusHandle {
  /// Checks if the widget or any descendant currently has focus visibility.
  ///
  /// Focus tracking must be explicitly enabled during widget creation by
  /// calling `MixBuiltin::trace_focus`. Without initialization, this method
  /// will consistently return `false` regardless of actual focus state.
  #[inline]
  pub fn is_focused(&self) -> bool { self.flags.read().contains(MixFlags::Focused) }

  /// Retrieves the reason for the most recent focus change event.
  ///
  /// When focused: indicates why focus was gained  
  /// When unfocused: indicates why focus was lost
  ///
  /// # Note
  /// Meaningful only when focus tracking is enabled via
  /// `MixBuiltin::trace_focus`. The return value becomes undefined if focus
  /// tracing wasn't initialized.
  pub fn focus_changed_reason(&self) -> FocusReason { self.flags.read().focus_changed_reason() }

  /// Indicates whether auto-focus is enabled for initial view activation.
  #[inline]
  pub fn auto_focus(&self) -> bool { self.flags.read().auto_focus() }

  /// Configures auto-focus behavior with proper flag management.
  ///
  /// Enables/disables automatic focus acquisition during view activation,
  /// maintaining valid flag combinations.
  pub fn set_auto_focus(&mut self, enable: bool) { self.flags.write().set_auto_focus(enable); }

  /// Retrieves validated tab index for keyboard navigation.
  ///
  /// Returns `None` if tab navigation is disabled (Focus flag unset).
  /// The returned value is guaranteed to be within valid bounds.
  pub fn tab_index(&self) -> Option<i16> { self.flags.read().tab_index() }

  /// Updates tab index with value sanitization and layout invalidation.
  ///
  /// Automatically enables focus tracking and clamps values to valid ranges.
  pub fn set_tab_index(&mut self, tab_idx: i16) { self.flags.write().set_tab_index(tab_idx); }

  pub fn request_focus(&self, reason: FocusReason) {
    if let Some(wnd) = AppCtx::get_window(self.wnd)
      && let Some(wid) = self.host.get()
    {
      wnd.focus_mgr.borrow_mut().focus(wid, reason);
    }
  }

  pub fn unfocus(&self, reason: FocusReason) {
    if let Some(wnd) = AppCtx::get_window(self.wnd) {
      let mut focus_mgr = wnd.focus_mgr.borrow_mut();
      if focus_mgr.focusing() == self.host.get() {
        focus_mgr.blur(reason);
      }
    }
  }
}

impl<'c> ComposeChild<'c> for MixBuiltin {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, mut child: Self::Child) -> Widget<'c> {
    let mix = this.read();
    if mix.contain_flag(MixFlags::Focus) {
      child = callbacks_for_focus_node(child);
    }
    if !mix.subject.is_closed() {
      let subject = mix.subject.clone();
      mix.on_disposed(move |_| {
        AppCtx::spawn_local(async move { subject.complete() });
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
    Self { flags: Stateful::new(MixFlags::default()), subject: Local::subject_mut_ref() }
  }
}

impl Clone for MixBuiltin {
  fn clone(&self) -> Self {
    let flags = self.flags.clone_writer();
    Self { flags, subject: self.subject.clone() }
  }
}

impl Clone for FocusHandle {
  fn clone(&self) -> Self {
    let flags = self.flags.clone_writer();
    Self { flags, wnd: self.wnd, host: self.host.clone() }
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
      let mut pipe_w = FatObj::new( pipe! {
        $read(trigger);
        fn_widget! { @Void { on_performed_layout: move |_| {} }}
      });

      @(pipe_w) {
        on_performed_layout: move |_| *$write(w_outer_layout) +=1 ,
      }
    };

    let wnd = TestWindow::from_widget(mix_keep);
    wnd.draw_frame();
    assert_eq!(*outer_layout.read(), 1);

    *w_trigger.write() = 1;

    wnd.draw_frame();
    assert_eq!(*outer_layout.read(), 2);
  }

  #[test]
  fn flags_data_check() {
    let mut mix = MixFlags::default();

    mix.set_focus_reason(FocusReason::Keyboard);
    mix.set_focus_reason(FocusReason::Pointer);
    mix.set_tab_index(16);
    mix.set_tab_index(-10);

    assert_eq!(mix.focus_changed_reason(), FocusReason::Pointer);
    assert_eq!(mix.tab_index(), Some(-10));
  }
}
