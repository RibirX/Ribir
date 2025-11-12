use ribir_core::prelude::*;

use crate::prelude::*;

/// The `Checkbox` allows users to toggle an option on or off, or represent a
/// list of options that are partially selected.
///
/// It also supports associating a label with the checkbox, the label inheriting
/// the text style from its nearest ancestor. The label can be positioned before
/// or after the radio button using the `Leading` and `Trailing` types, with the
/// default position being after the radio button.
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _check = checkbox! { checked: true };
///
/// let _partially = checkbox! { indeterminate: true };
/// ```
///
/// It also supports a label before or after the checkbox.
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let _check = checkbox! {
///   @ { "Default label placed after the checkbox!" }
/// };
///
/// let _heading = checkbox! {
///   @Leading::new("Label placed before the checkbox!")
/// };
///
/// let _trailing = checkbox! {
///   @Trailing::new("Label placed after the checkbox!")
/// };
/// ```
#[derive(Clone, Copy, Declare, PartialEq, Eq)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckState {
  pub checked: bool,
}

pub type CheckboxEvent = CustomEvent<CheckState>;

class_names! {
  /// The class name for the container of a checked checkbox.
  CHECKBOX_CHECKED,
  /// The class name for the container of an unchecked checkbox.
  CHECKBOX_UNCHECKED,
  /// The class name for the container of an indeterminate checkbox.
  CHECKBOX_INDETERMINATE,
  /// The base class name for the checkbox container.
  CHECKBOX,
  /// The class name for the icon of an unchecked checkbox.
  CHECKBOX_UNCHECKED_ICON,
  /// The class name for the icon of a checked checkbox.
  CHECKBOX_CHECKED_ICON,
  /// The class name for the icon of an indeterminate checkbox.
  CHECKBOX_INDETERMINATE_ICON
}

impl Checkbox {
  pub fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }

  fn state_class_name(&self) -> ClassName {
    if self.indeterminate {
      CHECKBOX_INDETERMINATE
    } else if self.checked {
      CHECKBOX_CHECKED
    } else {
      CHECKBOX_UNCHECKED
    }
  }

  fn icon_class_name(&self) -> ClassName {
    if self.indeterminate {
      CHECKBOX_INDETERMINATE_ICON
    } else if self.checked {
      CHECKBOX_CHECKED_ICON
    } else {
      CHECKBOX_UNCHECKED_ICON
    }
  }
}

impl ComposeChild<'static> for Checkbox {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fn_widget! {
      let mut obj = @FatObj {
        on_tap: move |e| {
          $write(this).switch_check();
          e.stop_propagation();
        },
        @ {
          let classes = class_array![distinct_pipe!($read(this).state_class_name()), CHECKBOX];
          let icon = @(classes) {
            @Icon {
              on_key_up: move |k| if *k.key() == VirtualKey::Named(NamedKey::Space) {
                $write(this).switch_check()
              },
              @Void { class: distinct_pipe!($read(this).icon_class_name())}
            }
          };
          icon_with_label(icon.into_widget(), child)
        }
      };

      let track_id = obj.track_id();
      let window = BuildCtx::get().window();
      watch!($read(this).checked)
        .distinct_until_changed()
        .subscribe(move |checked| {    
          if let Some(id) = track_id.get() { 
            window.bubble_custom_event(id, CheckState { checked });
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
    checkbox,
    WidgetTester::new(self::column! {
      @Checkbox { checked: true, @ { "checked" } }
      @Checkbox { indeterminate: true, @Leading::new("indeterminate") }
      @Checkbox { @Trailing::new("unchecked") }
    })
    .with_wnd_size(Size::new(240., 160.)),
  );
}
