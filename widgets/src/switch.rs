use ribir_core::prelude::*;

use crate::prelude::*;

/// The `Switch` widget toggles the state of a single item on or off.
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _switch = switch! { checked: true };
/// ```
///
/// It also supports a label before or after the switch.
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let _switch = switch! {
///   @ { "Default label placed after the switch!" }
/// };
///
/// let _leading = switch! {
///   @Leading::new("Label placed before the switch!")
/// };
///
/// let _trailing = switch! {
///   @Trailing::new("Label placed after the switch!")
/// };
/// ```
#[derive(Clone, Copy, Declare, PartialEq, Eq)]
pub struct Switch {
  #[declare(default, event = Switch.checked)]
  pub checked: bool,
}

pub type SwitchChanged = CustomEvent<Switch>;

class_names! {
  /// The class name for the container of a checked switch.
  SWITCH_CHECKED,
  /// The class name for the container of an unchecked switch.
  SWITCH_UNCHECKED,
  /// The base class name for the switch container.
  SWITCH,
  /// The base class name for the switch thumb.
  SWITCH_THUMB,
  /// The class name for the thumb of a checked switch.
  SWITCH_THUMB_CHECKED,
  /// The class name for the thumb of an unchecked switch.
  SWITCH_THUMB_UNCHECKED,
}

impl Switch {
  fn state_class_name(&self) -> ClassName {
    if self.checked { SWITCH_CHECKED } else { SWITCH_UNCHECKED }
  }

  fn thumb_class_name(&self) -> ClassName {
    if self.checked { SWITCH_THUMB_CHECKED } else { SWITCH_THUMB_UNCHECKED }
  }

  fn request_toggle(&self, e: &CommonEvent) {
    let mut new_state = *self;
    new_state.switch_check();
    e.window()
      .bubble_custom_event(e.target(), new_state);
  }

  /// Manually toggle the switch state.
  /// This is an imperative API and should not be used for default UI
  /// interactions.
  pub fn switch_check(&mut self) { self.checked = !self.checked; }
}

impl ComposeChild<'static> for Switch {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fn_widget! {
      let state_cls = distinct_pipe!($read(this).state_class_name());
      let thumb_state_cls = distinct_pipe!($read(this).thumb_class_name());
      let switch_widget = @ClassChain {
        class_chain: [SWITCH.r_into(), state_cls.r_into()],
        @Stack {
          @ClassChain {
            class_chain: [SWITCH_THUMB.r_into(), thumb_state_cls.r_into()],
            @Void {
              clamp: BoxClamp::fixed_size(Size::new(40., 20.)),
            }
          }
        }
      };

      @FatObj {
        cursor: CursorIcon::Pointer,
        on_action: move |e| $read(this).request_toggle(e),
        @ { icon_with_label(switch_widget.into_widget(), child) }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    switch,
    WidgetTester::new(self::column! {
      @Switch { checked: true, @ { "checked" } }
      @Switch { @Trailing::new("unchecked") }
    })
    .with_wnd_size(Size::new(240., 160.)),
  );
}
