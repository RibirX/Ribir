use ribir_core::prelude::*;

use crate::prelude::{PositionChild, icon_with_label};

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

class_names! {
  #[doc = "This base class specifies for the checkbox icon."]
  CHECKBOX,
  #[doc = "This class specifies the checked checkbox icon"]
  CHECKBOX_CHECKED,
  #[doc = "This class specifies the unchecked checkbox icon."]
  CHECKBOX_UNCHECKED,
  #[doc = "This class specifies the indeterminate checkbox icon."]
  CHECKBOX_INDETERMINATE,
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
}

impl ComposeChild<'static> for Checkbox {
  type Child = Option<PositionChild<TextInit>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    rdl! {
      let icon = @Class {
        class: distinct_pipe!($this.state_class_name()),
        @Void { class: CHECKBOX }
      };
      @FatObj {
        on_tap: move |_| $this.write().switch_check(),
        on_key_up: move |k| if *k.key() == VirtualKey::Named(NamedKey::Space) {
          $this.write().switch_check()
        },
        @ icon_with_label(icon.into_widget(), child)
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
  use crate::prelude::*;

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
