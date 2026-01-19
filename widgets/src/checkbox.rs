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

#[derive(Debug, Clone, Copy, Declare, PartialEq, Eq)]
pub struct Checkbox {
  #[declare(default, event = Checkbox.checked)]
  pub checked: bool,
  #[declare(default, event = Checkbox.indeterminate)]
  pub indeterminate: bool,
}

pub type CheckboxChanged = CustomEvent<Checkbox>;

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
  fn state_class_name(&self) -> ClassName {
    match (self.indeterminate, self.checked) {
      (true, _) => CHECKBOX_INDETERMINATE,
      (_, true) => CHECKBOX_CHECKED,
      _ => CHECKBOX_UNCHECKED,
    }
  }

  fn icon_class_name(&self) -> ClassName {
    match (self.indeterminate, self.checked) {
      (true, _) => CHECKBOX_INDETERMINATE_ICON,
      (_, true) => CHECKBOX_CHECKED_ICON,
      _ => CHECKBOX_UNCHECKED_ICON,
    }
  }

  fn request_toggle(&self, e: &CommonEvent) {
    let mut new_state = *self;
    new_state.switch_check();
    e.window()
      .bubble_custom_event(e.target(), new_state);
  }

  /// Manually toggle the checkbox state.
  /// This is an imperative API (Path C) and should not be used for default UI
  /// interactions.
  pub fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }
}

impl ComposeChild<'static> for Checkbox {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fn_widget! {
      let classes = class_array![distinct_pipe!($read(this).state_class_name()), CHECKBOX];
      let icon = @(classes) {
        @Icon { @Void { class: distinct_pipe!($read(this).icon_class_name())} }
      };
      let icon_with_label = icon_with_label(icon.into_widget(), child);
      @FatObj {
        on_action: move |e| $read(this).request_toggle(e),
        @{ icon_with_label }
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
    checkbox,
    WidgetTester::new(self::column! {
      @Checkbox { checked: true, @ { "checked" } }
      @Checkbox { indeterminate: true, @Leading::new("indeterminate") }
      @Checkbox { @Trailing::new("unchecked") }
    })
    .with_wnd_size(Size::new(240., 160.)),
  );
}
