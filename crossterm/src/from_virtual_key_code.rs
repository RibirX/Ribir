use crossterm::event::KeyCode as CrosstermVirtualKeyCode;
use ribir_core::prelude::VirtualKeyCode as RibirVirtualKeyCode;

pub struct WrappedVirtualKeyCode(CrosstermVirtualKeyCode);

impl From<CrosstermVirtualKeyCode> for WrappedVirtualKeyCode {
  fn from(value: CrosstermVirtualKeyCode) -> Self { WrappedVirtualKeyCode(value) }
}

impl From<WrappedVirtualKeyCode> for CrosstermVirtualKeyCode {
  fn from(val: WrappedVirtualKeyCode) -> Self { val.0 }
}

impl TryFrom<WrappedVirtualKeyCode> for RibirVirtualKeyCode {
  type Error = String;

  fn try_from(value: WrappedVirtualKeyCode) -> Result<Self, Self::Error> {
    match value.0 {
      CrosstermVirtualKeyCode::Insert => Ok(RibirVirtualKeyCode::Insert),
      CrosstermVirtualKeyCode::Home => Ok(RibirVirtualKeyCode::Home),
      CrosstermVirtualKeyCode::Delete => Ok(RibirVirtualKeyCode::Delete),
      CrosstermVirtualKeyCode::End => Ok(RibirVirtualKeyCode::End),
      CrosstermVirtualKeyCode::PageDown => Ok(RibirVirtualKeyCode::PageDown),
      CrosstermVirtualKeyCode::PageUp => Ok(RibirVirtualKeyCode::PageUp),

      CrosstermVirtualKeyCode::Left => Ok(RibirVirtualKeyCode::Left),
      CrosstermVirtualKeyCode::Up => Ok(RibirVirtualKeyCode::Up),
      CrosstermVirtualKeyCode::Right => Ok(RibirVirtualKeyCode::Right),
      CrosstermVirtualKeyCode::Down => Ok(RibirVirtualKeyCode::Down),

      CrosstermVirtualKeyCode::Backspace => Ok(RibirVirtualKeyCode::Back),
      CrosstermVirtualKeyCode::Esc => Ok(RibirVirtualKeyCode::Escape),
      CrosstermVirtualKeyCode::Enter => Ok(RibirVirtualKeyCode::Return),
      CrosstermVirtualKeyCode::Tab => Ok(RibirVirtualKeyCode::Tab),

      CrosstermVirtualKeyCode::BackTab => {
        Err("BackTab not implemented in VirtualKeyCode".to_owned())
      }
      CrosstermVirtualKeyCode::F(f) => match f {
        1 => Ok(RibirVirtualKeyCode::F1),
        2 => Ok(RibirVirtualKeyCode::F2),
        3 => Ok(RibirVirtualKeyCode::F3),
        4 => Ok(RibirVirtualKeyCode::F4),
        5 => Ok(RibirVirtualKeyCode::F5),
        6 => Ok(RibirVirtualKeyCode::F6),
        7 => Ok(RibirVirtualKeyCode::F7),
        8 => Ok(RibirVirtualKeyCode::F8),
        9 => Ok(RibirVirtualKeyCode::F9),
        10 => Ok(RibirVirtualKeyCode::F10),
        11 => Ok(RibirVirtualKeyCode::F11),
        12 => Ok(RibirVirtualKeyCode::F12),
        other => Err(format!("Unsupported event code {other:?}")),
        // CrosstermVirtualKeyCode::F13 => RibirVirtualKeyCode::F13,
        // CrosstermVirtualKeyCode::F14 => RibirVirtualKeyCode::F14,
        // CrosstermVirtualKeyCode::F15 => RibirVirtualKeyCode::F15,
        // CrosstermVirtualKeyCode::F16 => RibirVirtualKeyCode::F16,
        // CrosstermVirtualKeyCode::F17 => RibirVirtualKeyCode::F17,
        // CrosstermVirtualKeyCode::F18 => RibirVirtualKeyCode::F18,
        // CrosstermVirtualKeyCode::F19 => RibirVirtualKeyCode::F19,
        // CrosstermVirtualKeyCode::F20 => RibirVirtualKeyCode::F20,
        // CrosstermVirtualKeyCode::F21 => RibirVirtualKeyCode::F21,
        // CrosstermVirtualKeyCode::F22 => RibirVirtualKeyCode::F22,
        // CrosstermVirtualKeyCode::F23 => RibirVirtualKeyCode::F23,
        // CrosstermVirtualKeyCode::F24 => RibirVirtualKeyCode::F24,
      },

      // CrosstermVirtualKeyCode::Alt(c) => Err("Not yet implemented: Alt {c:?}"),
      // CrosstermVirtualKeyCode::Ctrl(c) => Err("Not yet implemented: Ctrl {c:?}"),
      CrosstermVirtualKeyCode::Null => Err("Not yet implemented (Null)".to_owned()),

      CrosstermVirtualKeyCode::Char(c) => match c {
        // '\x00' => RibirVirtualKeyCode::A, Null
        // '\x01' => RibirVirtualKeyCode::A, // Start of Heading
        // '\x02' => RibirVirtualKeyCode::A, // Start of Text
        // '\x03' => RibirVirtualKeyCode::A, // End of Text
        // '\x04' => RibirVirtualKeyCode::A, // End of Transmission
        // '\x05' => RibirVirtualKeyCode::A, // Enquiry
        // '\x06' => RibirVirtualKeyCode::A, // Acknowledgement
        // '\x07' => RibirVirtualKeyCode::A, // Bell
        // '\x08' => RibirVirtualKeyCode::A, // Backspace
        // '\x09' => RibirVirtualKeyCode::A, // Horizontlal Tab
        // '\x0A' => RibirVirtualKeyCode::A, // Linefeed
        // '\x0B' => RibirVirtualKeyCode::A, // Vertial Tab
        // '\x0C' => RibirVirtualKeyCode::A, // Form Feed
        // '\x0D' => RibirVirtualKeyCode::A, // Carriage Return
        // '\x0E' => RibirVirtualKeyCode::A, // Shift Out
        // '\x0F' => RibirVirtualKeyCode::A, // Shift In
        // '\x10' => RibirVirtualKeyCode::A, // Data Link Escape
        // '\x11' => RibirVirtualKeyCode::A, // Device Control 1
        // '\x12' => RibirVirtualKeyCode::A, // Device Control 2
        // '\x13' => RibirVirtualKeyCode::A, // Device Control 3
        // '\x14' => RibirVirtualKeyCode::A, // Device Control 4
        // '\x15' => RibirVirtualKeyCode::A, // Negative Acknowledgement
        // '\x16' => RibirVirtualKeyCode::A, // Synchronous Idle
        // '\x17' => RibirVirtualKeyCode::A, // End of Transmission Block
        // '\x18' => RibirVirtualKeyCode::A, // Cancel
        // '\x19' => RibirVirtualKeyCode::A, // End of Medium
        // '\x1A' => RibirVirtualKeyCode::A, // Substitute

        // '\x1C' => RibirVirtualKeyCode::A, // File Separator
        // '\x1D' => RibirVirtualKeyCode::A, // Group Separator
        // '\x1E' => RibirVirtualKeyCode::A, // Record Separator
        // '\x1F' => RibirVirtualKeyCode::A, // Unit Separator
        '\x20' => Ok(RibirVirtualKeyCode::Space), // Space
        // '\x21' => RibirVirtualKeyCode::A, // !
        // '\x22' => RibirVirtualKeyCode::A, // "
        // '\x23' => RibirVirtualKeyCode::A, // #
        // '\x24' => RibirVirtualKeyCode::A, // $
        // '\x25' => RibirVirtualKeyCode::A, // %
        // '\x26' => RibirVirtualKeyCode::A, // &
        '\x27' => Ok(RibirVirtualKeyCode::Apostrophe), // '
        // '\x28' => RibirVirtualKeyCode::A, // (
        // '\x29' => RibirVirtualKeyCode::A, // )
        '\x2A' => Ok(RibirVirtualKeyCode::Asterisk), // *
        '\x2B' => Ok(RibirVirtualKeyCode::Plus),     // +
        '\x2C' => Ok(RibirVirtualKeyCode::Comma),    // ,
        '\x2D' => Ok(RibirVirtualKeyCode::Minus),    // -
        // '\x2E' => RibirVirtualKeyCode::A, // .
        '\x2F' => Ok(RibirVirtualKeyCode::Slash), // /
        '\x30' => Ok(RibirVirtualKeyCode::Key0),  // 0

        '\x31' => Ok(RibirVirtualKeyCode::Key1),      // 1
        '\x32' => Ok(RibirVirtualKeyCode::Key2),      // 2
        '\x33' => Ok(RibirVirtualKeyCode::Key3),      // 3
        '\x34' => Ok(RibirVirtualKeyCode::Key4),      // 4
        '\x35' => Ok(RibirVirtualKeyCode::Key5),      // 5
        '\x36' => Ok(RibirVirtualKeyCode::Key6),      // 6
        '\x37' => Ok(RibirVirtualKeyCode::Key7),      // 7
        '\x38' => Ok(RibirVirtualKeyCode::Key8),      // 8
        '\x39' => Ok(RibirVirtualKeyCode::Key9),      // 9
        '\x3A' => Ok(RibirVirtualKeyCode::Colon),     // :
        '\x3B' => Ok(RibirVirtualKeyCode::Semicolon), // ;
        // '\x3C' => RibirVirtualKeyCode::A, // <
        '\x3D' => Ok(RibirVirtualKeyCode::Equals), // =
        // '\x3E' => RibirVirtualKeyCode::A, // >
        // '\x3F' => RibirVirtualKeyCode::A, // ?
        '\x40' => Ok(RibirVirtualKeyCode::Grave), // ` | @
        '\x41' => Ok(RibirVirtualKeyCode::A),     // A
        '\x42' => Ok(RibirVirtualKeyCode::B),     // B
        '\x43' => Ok(RibirVirtualKeyCode::C),     // C
        '\x44' => Ok(RibirVirtualKeyCode::D),     // D
        '\x45' => Ok(RibirVirtualKeyCode::E),     // E
        '\x46' => Ok(RibirVirtualKeyCode::F),     // F
        '\x47' => Ok(RibirVirtualKeyCode::G),     // G
        '\x48' => Ok(RibirVirtualKeyCode::H),     // H
        '\x49' => Ok(RibirVirtualKeyCode::I),     // I
        '\x4A' => Ok(RibirVirtualKeyCode::J),     // J
        '\x4B' => Ok(RibirVirtualKeyCode::K),     // K
        '\x4C' => Ok(RibirVirtualKeyCode::L),     // L
        '\x4D' => Ok(RibirVirtualKeyCode::M),     // M
        '\x4E' => Ok(RibirVirtualKeyCode::N),     // N
        '\x4F' => Ok(RibirVirtualKeyCode::O),     // O

        '\x50' => Ok(RibirVirtualKeyCode::P),         // P
        '\x51' => Ok(RibirVirtualKeyCode::Q),         // Q
        '\x52' => Ok(RibirVirtualKeyCode::R),         // R
        '\x53' => Ok(RibirVirtualKeyCode::S),         // S
        '\x54' => Ok(RibirVirtualKeyCode::T),         // T
        '\x55' => Ok(RibirVirtualKeyCode::U),         // U
        '\x56' => Ok(RibirVirtualKeyCode::V),         // V
        '\x57' => Ok(RibirVirtualKeyCode::W),         // W
        '\x58' => Ok(RibirVirtualKeyCode::X),         // X
        '\x59' => Ok(RibirVirtualKeyCode::Y),         // Y
        '\x5A' => Ok(RibirVirtualKeyCode::Z),         // Z
        '\x5B' => Ok(RibirVirtualKeyCode::LBracket),  // [
        '\x5C' => Ok(RibirVirtualKeyCode::Backslash), // \
        '\x5D' => Ok(RibirVirtualKeyCode::RBracket),  // ]
        '\x5E' => Ok(RibirVirtualKeyCode::Caret),     // ^
        '\x5F' => Ok(RibirVirtualKeyCode::Underline), // _

        '\x60' => Ok(RibirVirtualKeyCode::At), // @ | `
        '\x61' => Ok(RibirVirtualKeyCode::A),  // a
        '\x62' => Ok(RibirVirtualKeyCode::B),  // b
        '\x63' => Ok(RibirVirtualKeyCode::C),  // c
        '\x64' => Ok(RibirVirtualKeyCode::D),  // d
        '\x65' => Ok(RibirVirtualKeyCode::E),  // e
        '\x66' => Ok(RibirVirtualKeyCode::F),  // f
        '\x67' => Ok(RibirVirtualKeyCode::G),  // g
        '\x68' => Ok(RibirVirtualKeyCode::H),  // h
        '\x69' => Ok(RibirVirtualKeyCode::I),  // i
        '\x6A' => Ok(RibirVirtualKeyCode::J),  // j
        '\x6B' => Ok(RibirVirtualKeyCode::K),  // k
        '\x6C' => Ok(RibirVirtualKeyCode::L),  // l
        '\x6D' => Ok(RibirVirtualKeyCode::M),  // m
        '\x6E' => Ok(RibirVirtualKeyCode::N),  // n
        '\x6F' => Ok(RibirVirtualKeyCode::O),  // o

        '\x70' => Ok(RibirVirtualKeyCode::P), // p
        '\x71' => Ok(RibirVirtualKeyCode::Q), // q
        '\x72' => Ok(RibirVirtualKeyCode::R), // r
        '\x73' => Ok(RibirVirtualKeyCode::S), // s
        '\x74' => Ok(RibirVirtualKeyCode::T), // t
        '\x75' => Ok(RibirVirtualKeyCode::U), // u
        '\x76' => Ok(RibirVirtualKeyCode::V), // v
        '\x77' => Ok(RibirVirtualKeyCode::W), // w
        '\x78' => Ok(RibirVirtualKeyCode::X), // x
        '\x79' => Ok(RibirVirtualKeyCode::Y), // y
        '\x7A' => Ok(RibirVirtualKeyCode::Z), // z
        '\x7B' => Ok(RibirVirtualKeyCode::A), // {
        '\x7C' => Ok(RibirVirtualKeyCode::A), // |
        '\x7D' => Ok(RibirVirtualKeyCode::A), // }
        '\x7E' => Ok(RibirVirtualKeyCode::A), // ~

        other => Err(format!("Unsupported event code {other:?}")),
      },

      // CrosstermVirtualKeyCode::Snapshot => RibirVirtualKeyCode::Snapshot,
      // CrosstermVirtualKeyCode::Scroll => RibirVirtualKeyCode::Scroll,
      CrosstermVirtualKeyCode::Pause => Ok(RibirVirtualKeyCode::Pause),

      // CrosstermVirtualKeyCode::Compose => RibirVirtualKeyCode::Compose,
      CrosstermVirtualKeyCode::NumLock => Ok(RibirVirtualKeyCode::Numlock),
      // CrosstermVirtualKeyCode::Numpad0 => RibirVirtualKeyCode::Numpad0,
      // CrosstermVirtualKeyCode::Numpad1 => RibirVirtualKeyCode::Numpad1,
      // CrosstermVirtualKeyCode::Numpad2 => RibirVirtualKeyCode::Numpad2,
      // CrosstermVirtualKeyCode::Numpad3 => RibirVirtualKeyCode::Numpad3,
      // CrosstermVirtualKeyCode::Numpad4 => RibirVirtualKeyCode::Numpad4,
      // CrosstermVirtualKeyCode::Numpad5 => RibirVirtualKeyCode::Numpad5,
      // CrosstermVirtualKeyCode::Numpad6 => RibirVirtualKeyCode::Numpad6,
      // CrosstermVirtualKeyCode::Numpad7 => RibirVirtualKeyCode::Numpad7,
      // CrosstermVirtualKeyCode::Numpad8 => RibirVirtualKeyCode::Numpad8,
      // CrosstermVirtualKeyCode::Numpad9 => RibirVirtualKeyCode::Numpad9,
      // CrosstermVirtualKeyCode::NumpadAdd => RibirVirtualKeyCode::NumpadAdd,
      // CrosstermVirtualKeyCode::NumpadDivide => RibirVirtualKeyCode::NumpadDivide,
      // CrosstermVirtualKeyCode::NumpadDecimal => RibirVirtualKeyCode::NumpadDecimal,
      // CrosstermVirtualKeyCode::NumpadComma => RibirVirtualKeyCode::NumpadComma,
      // CrosstermVirtualKeyCode::NumpadEnter => RibirVirtualKeyCode::NumpadEnter,
      // CrosstermVirtualKeyCode::NumpadEquals => RibirVirtualKeyCode::NumpadEquals,
      // CrosstermVirtualKeyCode::NumpadMultiply => RibirVirtualKeyCode::NumpadMultiply,
      // CrosstermVirtualKeyCode::NumpadSubtract => RibirVirtualKeyCode::NumpadSubtract,

      // CrosstermVirtualKeyCode::AbntC1 => RibirVirtualKeyCode::AbntC1,
      // CrosstermVirtualKeyCode::AbntC2 => RibirVirtualKeyCode::AbntC2,
      // CrosstermVirtualKeyCode::Apps => RibirVirtualKeyCode::Apps,
      // CrosstermVirtualKeyCode::Ax => RibirVirtualKeyCode::Ax,
      // CrosstermVirtualKeyCode::Calculator => RibirVirtualKeyCode::Calculator,
      // CrosstermVirtualKeyCode::Capital => RibirVirtualKeyCode::Capital,
      // CrosstermVirtualKeyCode::Convert => RibirVirtualKeyCode::Convert,
      // CrosstermVirtualKeyCode::Kana => RibirVirtualKeyCode::Kana,
      // CrosstermVirtualKeyCode::Kanji => RibirVirtualKeyCode::Kanji,
      // CrosstermVirtualKeyCode::LAlt => RibirVirtualKeyCode::LAlt,
      // CrosstermVirtualKeyCode::LControl => RibirVirtualKeyCode::LControl,
      // CrosstermVirtualKeyCode::LShift => RibirVirtualKeyCode::LShift,
      // CrosstermVirtualKeyCode::LWin => RibirVirtualKeyCode::LWin,
      // CrosstermVirtualKeyCode::Mail => RibirVirtualKeyCode::Mail,
      // CrosstermVirtualKeyCode::MediaSelect => RibirVirtualKeyCode::MediaSelect,
      // CrosstermVirtualKeyCode::MediaStop => RibirVirtualKeyCode::MediaStop,
      // CrosstermVirtualKeyCode::Mute => RibirVirtualKeyCode::Mute,
      // CrosstermVirtualKeyCode::MyComputer => RibirVirtualKeyCode::MyComputer,
      // CrosstermVirtualKeyCode::NavigateForward => RibirVirtualKeyCode::NavigateForward,
      // CrosstermVirtualKeyCode::NavigateBackward => RibirVirtualKeyCode::NavigateBackward,
      // CrosstermVirtualKeyCode::NextTrack => RibirVirtualKeyCode::NextTrack,
      // CrosstermVirtualKeyCode::NoConvert => RibirVirtualKeyCode::NoConvert,
      // CrosstermVirtualKeyCode::OEM102 => RibirVirtualKeyCode::OEM102,
      // CrosstermVirtualKeyCode::Period => RibirVirtualKeyCode::Period,
      // CrosstermVirtualKeyCode::PlayPause => RibirVirtualKeyCode::PlayPause,
      // CrosstermVirtualKeyCode::Power => RibirVirtualKeyCode::Power,
      // CrosstermVirtualKeyCode::PrevTrack => RibirVirtualKeyCode::PrevTrack,
      // CrosstermVirtualKeyCode::RAlt => RibirVirtualKeyCode::RAlt,
      // CrosstermVirtualKeyCode::RControl => RibirVirtualKeyCode::RControl,
      // CrosstermVirtualKeyCode::RShift => RibirVirtualKeyCode::RShift,
      // CrosstermVirtualKeyCode::RWin => RibirVirtualKeyCode::RWin,
      // CrosstermVirtualKeyCode::Sleep => RibirVirtualKeyCode::Sleep,
      // CrosstermVirtualKeyCode::Stop => RibirVirtualKeyCode::Stop,
      // CrosstermVirtualKeyCode::Sysrq => RibirVirtualKeyCode::Sysrq,

      // CrosstermVirtualKeyCode::Unlabeled => RibirVirtualKeyCode::Unlabeled,
      // CrosstermVirtualKeyCode::VolumeDown => RibirVirtualKeyCode::VolumeDown,
      // CrosstermVirtualKeyCode::VolumeUp => RibirVirtualKeyCode::VolumeUp,
      // CrosstermVirtualKeyCode::Wake => RibirVirtualKeyCode::Wake,
      // CrosstermVirtualKeyCode::WebBack => RibirVirtualKeyCode::WebBack,
      // CrosstermVirtualKeyCode::WebFavorites => RibirVirtualKeyCode::WebFavorites,
      // CrosstermVirtualKeyCode::WebForward => RibirVirtualKeyCode::WebForward,
      // CrosstermVirtualKeyCode::WebHome => RibirVirtualKeyCode::WebHome,
      // CrosstermVirtualKeyCode::WebRefresh => RibirVirtualKeyCode::WebRefresh,
      // CrosstermVirtualKeyCode::WebSearch => RibirVirtualKeyCode::WebSearch,
      // CrosstermVirtualKeyCode::WebStop => RibirVirtualKeyCode::WebStop,
      // CrosstermVirtualKeyCode::Yen => RibirVirtualKeyCode::Yen,
      // CrosstermVirtualKeyCode::Copy => RibirVirtualKeyCode::Copy,
      // CrosstermVirtualKeyCode::Paste => RibirVirtualKeyCode::Paste,
      // CrosstermVirtualKeyCode::Cut => RibirVirtualKeyCode::Cut,
      other => Err(format!("Unsupported event code {other:?}")),
    }
  }
}
