use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_query_self_only, prelude::*,
};

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

#[derive(Declare)]
pub struct KeyDownListener {
  #[declare(builtin, default, convert=custom)]
  on_key_down: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

#[derive(Declare)]
pub struct KeyUpListener {
  #[declare(
    builtin,
    convert=custom
  )]
  on_key_up: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

impl_listener!(
  KeyDownListener,
  KeyDownListenerDeclarer,
  on_key_down,
  KeyboardEvent,
  key_down_stream
);
impl_compose_child_with_focus_for_listener!(KeyDownListener);

impl_listener!(
  KeyUpListener,
  KeyUpListenerDeclarer,
  on_key_up,
  KeyboardEvent,
  key_up_stream
);

impl_compose_child_with_focus_for_listener!(KeyUpListener);

impl std::borrow::Borrow<EventCommon> for KeyboardEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for KeyboardEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for KeyboardEvent {
  type Target = EventCommon;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for KeyboardEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

/// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum VirtualKeyCode {
  /// The '1' key over the letters.
  Key1,
  /// The '2' key over the letters.
  Key2,
  /// The '3' key over the letters.
  Key3,
  /// The '4' key over the letters.
  Key4,
  /// The '5' key over the letters.
  Key5,
  /// The '6' key over the letters.
  Key6,
  /// The '7' key over the letters.
  Key7,
  /// The '8' key over the letters.
  Key8,
  /// The '9' key over the letters.
  Key9,
  /// The '0' key over the 'O' and 'P' keys.
  Key0,

  A,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
  K,
  L,
  M,
  N,
  O,
  P,
  Q,
  R,
  S,
  T,
  U,
  V,
  W,
  X,
  Y,
  Z,

  /// The Escape key, next to F1.
  Escape,

  F1,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,
  F11,
  F12,
  F13,
  F14,
  F15,
  F16,
  F17,
  F18,
  F19,
  F20,
  F21,
  F22,
  F23,
  F24,

  /// Print Screen/SysRq.
  Snapshot,
  /// Scroll Lock.
  Scroll,
  /// Pause/Break key, next to Scroll lock.
  Pause,

  /// `Insert`, next to Backspace.
  Insert,
  Home,
  Delete,
  End,
  PageDown,
  PageUp,

  Left,
  Up,
  Right,
  Down,

  /// The Backspace key, right over Enter.
  // TODO: rename
  Back,
  /// The Enter key.
  Return,
  /// The space bar.
  Space,

  /// The "Compose" key on Linux.
  Compose,

  Caret,

  Numlock,
  Numpad0,
  Numpad1,
  Numpad2,
  Numpad3,
  Numpad4,
  Numpad5,
  Numpad6,
  Numpad7,
  Numpad8,
  Numpad9,
  NumpadAdd,
  NumpadDivide,
  NumpadDecimal,
  NumpadComma,
  NumpadEnter,
  NumpadEquals,
  NumpadMultiply,
  NumpadSubtract,

  AbntC1,
  AbntC2,
  Apostrophe,
  Apps,
  Asterisk,
  At,
  Ax,
  Backslash,
  Calculator,
  Capital,
  Colon,
  Comma,
  Convert,
  Equals,
  Grave,
  Kana,
  Kanji,
  LAlt,
  LBracket,
  LControl,
  LShift,
  LWin,
  Mail,
  MediaSelect,
  MediaStop,
  Minus,
  Mute,
  MyComputer,
  // also called "Next"
  NavigateForward,
  // also called "Prior"
  NavigateBackward,
  NextTrack,
  NoConvert,
  OEM102,
  Period,
  PlayPause,
  Plus,
  Power,
  PrevTrack,
  RAlt,
  RBracket,
  RControl,
  RShift,
  RWin,
  Semicolon,
  Slash,
  Sleep,
  Stop,
  Sysrq,
  Tab,
  Underline,
  Unlabeled,
  VolumeDown,
  VolumeUp,
  Wake,
  WebBack,
  WebFavorites,
  WebForward,
  WebHome,
  WebRefresh,
  WebSearch,
  WebStop,
  Yen,
  Copy,
  Paste,
  Cut,
}

pub type ScanCode = u32;

bitflags! {
    /// Represents the current state of the keyboard modifiers
    ///
    /// Each flag represents a modifier and is set if this modifier is active.
    #[derive(Default)]
    pub struct ModifiersState: u32 {
        // left and right modifiers are currently commented out, but we should be able to support
        // them in a future release
        /// The "shift" key.
        const SHIFT = 0b100;
        // const LSHIFT = 0b010;
        // const RSHIFT = 0b001;
        /// The "control" key.
        const CTRL = 0b100 << 3;
        // const LCTRL = 0b010 << 3;
        // const RCTRL = 0b001 << 3;
        /// The "alt" key.
        const ALT = 0b100 << 6;
        // const LALT = 0b010 << 6;
        // const RALT = 0b001 << 6;
        /// This is the "windows" key on PC and "command" key on Mac.
        const LOGO = 0b100 << 9;
        // const LLOGO = 0b010 << 9;
        // const RLOGO = 0b001 << 9;
    }
}

impl ModifiersState {
  /// Returns `true` if the shift key is pressed.
  pub fn shift(&self) -> bool { self.intersects(Self::SHIFT) }
  /// Returns `true` if the control key is pressed.
  pub fn ctrl(&self) -> bool { self.intersects(Self::CTRL) }
  /// Returns `true` if the alt key is pressed.
  pub fn alt(&self) -> bool { self.intersects(Self::ALT) }
  /// Returns `true` if the logo key is pressed.
  pub fn logo(&self) -> bool { self.intersects(Self::LOGO) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;
  use std::{cell::RefCell, rc::Rc};
  // use winit::event::{DeviceId, ElementState, KeyboardInput, WindowEvent};

  fn new_key_event(key: VirtualKeyCode, state: ElementState) -> WindowEvent /*<'static>*/ {
    #[allow(deprecated)]
    WindowEvent::KeyboardInput {
      device_id: MockPointerId::zero(),
      input: KeyboardInput {
        scancode: 0,
        virtual_keycode: Some(key),
        state,
      },
      is_synthetic: false,
    }
  }

  #[test]
  fn smoke() {
    #[derive(Default)]
    struct Keys(Rc<RefCell<Vec<String>>>);

    impl Compose for Keys {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: Size::zero(),
            auto_focus: true,
            on_key_down: move |key| {
              this.0
                .borrow_mut()
                .push(format!("key down {:?}", key.key));
            },
            on_key_up: move |key| {
              this.0.borrow_mut().push(format!("key up {:?}", key.key));
            }
          }
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = Window::default_mock(w.into_widget(), None);
    wnd.draw_frame();

    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Pressed));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Released));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Pressed));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Released));

    assert_eq!(
      &*keys.borrow(),
      &[
        "key down Key0",
        "key up Key0",
        "key down Key1",
        "key up Key1"
      ]
    );
  }
}
