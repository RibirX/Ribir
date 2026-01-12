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
  #[declare(default)]
  pub checked: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SwitchState {
  pub checked: bool,
}

pub type SwitchEvent = CustomEvent<SwitchState>;

class_names! {
  /// The class name for the container of a checked switch.
  SWITCH_CHECKED,
  /// The class name for the container of an unchecked switch.
  SWITCH_UNCHECKED,
  /// The class name for the thumb of a checked switch.
  SWITCH_THUMB_CHECKED,
  /// The class name for the thumb of an unchecked switch.
  SWITCH_THUMB_UNCHECKED,
}

impl Switch {
  pub fn switch_check(&mut self) { self.checked = !self.checked; }

  fn state_class_name(&self) -> ClassName {
    if self.checked { SWITCH_CHECKED } else { SWITCH_UNCHECKED }
  }

  fn thumb_class_name(&self) -> ClassName {
    if self.checked { SWITCH_THUMB_CHECKED } else { SWITCH_THUMB_UNCHECKED }
  }
}

impl ComposeChild<'static> for Switch {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fn_widget! {
      let mut obj = @FatObj {
        on_action: move |e| {
          $write(this).switch_check();
          e.stop_propagation();
        },
        @ {
          let switch_widget = @Container {
            size: Size::new(28., 20.),
            class: distinct_pipe!($read(this).state_class_name()),
            @Void { class: distinct_pipe!($read(this).thumb_class_name()), }
          };
          icon_with_label(switch_widget.into_widget(), child)
        }
      };

      let track_id = obj.track_id();
      let window = BuildCtx::get().window();
      watch!($read(this).checked)
        .distinct_until_changed()
        .subscribe(move |checked| {
          if let Some(id) = track_id.get() {
            window.bubble_custom_event(id, SwitchState { checked });
          }
        });

      obj
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
