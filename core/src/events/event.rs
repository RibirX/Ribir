use ribir_painter::{DeviceOffset, DevicePoint, DeviceSize};
use std::fmt::Debug;

use super::{ModifiersState, MouseButtons, PointerId, ScanCode, VirtualKeyCode};

#[derive(Debug)]
pub enum WindowEvent {
  Unsupported,
  Resized(DeviceSize),

  /// The window received a unicode character.
  ///
  /// See also the [`Ime`](Self::Ime) event for more complex character
  /// sequences.
  ReceivedCharacter(char),

  /// An event from the keyboard has been received.
  KeyboardInput {
    device_id: Box<dyn PointerId>,
    input: KeyboardInput,
    /// If `true`, the event was generated synthetically by winit
    /// in one of the following circumstances:
    ///
    /// * Synthetic key press events are generated for all keys pressed when a
    ///   window gains focus. Likewise, synthetic key release events are
    ///   generated for all keys pressed when a window goes out of focus.
    ///   ***Currently, this is only functional on X11 and Windows***
    ///
    /// Otherwise, this value is always `false`.
    is_synthetic: bool,
  },

  ModifiersChanged(ModifiersState),
  CursorMoved {
    device_id: Box<dyn PointerId>,

    /// (x,y) coords in pixels relative to the top-left corner of the window.
    /// Because the range of this data is limited by the display area and it
    /// may have been transformed by the OS to implement effects such as cursor
    /// acceleration, it should not be used to implement non-cursor-like
    /// interactions such as 3D camera control.
    position: DevicePoint,
  },

  /// The cursor has left the window.
  CursorLeft {
    device_id: Box<dyn PointerId>,
  },

  /// A mouse wheel movement or touchpad scroll occurred.
  MouseWheel {
    device_id: Box<dyn PointerId>,
    delta: MouseScrollDelta,
    phase: TouchPhase,
  },

  /// An mouse button press has been received.
  MouseInput {
    device_id: Box<dyn PointerId>,
    state: ElementState,
    button: MouseButtons,
  },

  /// The window's scale factor has changed.
  ///
  /// The following user actions can cause DPI changes:
  ///
  /// * Changing the display's resolution.
  /// * Changing the display's scale factor (e.g. in Control Panel on Windows).
  /// * Moving the window to a display with a different scale factor.
  ///
  /// After this event callback has been processed, the window will be resized
  /// to whatever value is pointed to by the `new_inner_size` reference. By
  /// default, this will contain the size suggested by the OS, but it can be
  /// changed to any value.
  ///
  /// For more information about DPI in general, see the [`dpi`](crate::dpi)
  /// module.
  ScaleFactorChanged {
    scale_factor: f64,
    new_inner_size: DeviceSize,
  },
}

impl PartialEq for WindowEvent {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Resized(l0), Self::Resized(r0)) => l0 == r0,
      (Self::ReceivedCharacter(l0), Self::ReceivedCharacter(r0)) => l0 == r0,
      (
        Self::KeyboardInput {
          device_id: l_device_id,
          input: l_input,
          is_synthetic: l_is_synthetic,
        },
        Self::KeyboardInput {
          device_id: r_device_id,
          input: r_input,
          is_synthetic: r_is_synthetic,
        },
      ) => l_device_id == r_device_id && l_input == r_input && l_is_synthetic == r_is_synthetic,
      (Self::ModifiersChanged(l0), Self::ModifiersChanged(r0)) => l0 == r0,
      (
        Self::CursorMoved {
          device_id: l_device_id,
          position: l_position,
        },
        Self::CursorMoved {
          device_id: r_device_id,
          position: r_position,
        },
      ) => l_device_id == r_device_id && l_position == r_position,
      (
        Self::CursorLeft { device_id: l_device_id },
        Self::CursorLeft { device_id: r_device_id },
      ) => l_device_id == r_device_id,
      (
        Self::MouseWheel {
          device_id: l_device_id,
          delta: l_delta,
          phase: l_phase,
        },
        Self::MouseWheel {
          device_id: r_device_id,
          delta: r_delta,
          phase: r_phase,
        },
      ) => l_device_id == r_device_id && l_delta == r_delta && l_phase == r_phase,
      (
        Self::MouseInput {
          device_id: l_device_id,
          state: l_state,
          button: l_button,
        },
        Self::MouseInput {
          device_id: r_device_id,
          state: r_state,
          button: r_button,
        },
      ) => l_device_id == r_device_id && l_state == r_state && l_button == r_button,
      (
        Self::ScaleFactorChanged {
          scale_factor: l_scale_factor,
          new_inner_size: l_new_inner_size,
        },
        Self::ScaleFactorChanged {
          scale_factor: r_scale_factor,
          new_inner_size: r_new_inner_size,
        },
      ) => l_scale_factor == r_scale_factor && l_new_inner_size == r_new_inner_size,
      _ => false,
    }
  }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ElementState {
  Pressed,
  Released,
}

/// Describes a difference in the mouse scroll wheel state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseScrollDelta {
  /// Amount in lines or rows to scroll in the horizontal
  /// and vertical directions.
  ///
  /// Positive values indicate that the content that is being scrolled should
  /// move right and down (revealing more content left and up).
  LineDelta(f32, f32),

  /// Amount in pixels to scroll in the horizontal and
  /// vertical direction.
  ///
  /// Scroll events are expressed as a `PixelDelta` if
  /// supported by the device (eg. a touchpad) and
  /// platform.
  ///
  /// Positive values indicate that the content being scrolled should
  /// move right/down.
  ///
  /// For a 'natural scrolling' touch pad (that acts like a touch screen)
  /// this means moving your fingers right and down should give positive values,
  /// and move the content right and down (to reveal more things left and up).
  PixelDelta(DeviceOffset),
}

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TouchPhase {
  Started,
  Moved,
  Ended,
  Cancelled,
}

/// Describes a keyboard input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardInput {
  /// Identifies the physical key pressed
  ///
  /// This should not change if the user adjusts the host's keyboard map. Use
  /// when the physical location of the key is more important than the key's
  /// host GUI semantics, such as for movement controls in a first-person
  /// game.
  pub scancode: ScanCode,

  pub state: ElementState,

  /// Identifies the semantic meaning of the key
  ///
  /// Use when the semantics of the key are more important than the physical
  /// location of the key, such as when implementing appropriate behavior for
  /// "page up."
  pub virtual_keycode: Option<VirtualKeyCode>,
}
