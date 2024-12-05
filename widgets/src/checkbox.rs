use ribir_core::prelude::*;
use svg::named_svgs;

use crate::{
  common_widget::{Leading, Trailing},
  prelude::{Icon, Row, Text},
  text::TextInit,
};

class_names! {
  #[doc = "This base class specifies for the checkbox widget."]
  CHECKBOX,
  #[doc = "This class specifies the checked checkbox."]
  CHECKBOX_CHECKED,
  #[doc = "This class specifies the unchecked checkbox."]
  CHECKBOX_UNCHECKED,
  #[doc = "This class specifies the indeterminate checkbox."]
  CHECKBOX_INDETERMINATE,
}
/// The `Checkbox` allows users to toggle an option on or off, or represent a
/// list of options that are partially selected.
///
/// It also supports associating a label with the checkbox, with the label
/// inheriting the text style from its surrounding context.
///
/// # Example
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
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
///   @Leading("Label placed before the checkbox!")
/// };
///
/// let _trailing = checkbox! {
///   @Trailing("Label placed after the checkbox!")
/// };
/// ```
///
/// If you are a theme maker, you can register three named SVGs that the
/// checkbox uses with the following constants:
///
/// * `UNCHECKED_ICON`
/// * `CHECKED_ICON`
/// * `INDETERMINATE_ICON`
///
/// This allows you to quickly customize the appearance of a checkbox to your
/// preferences.
#[derive(Clone, Copy, Declare, PartialEq, Eq)]
pub struct Checkbox {
  #[declare(default)]
  pub checked: bool,
  #[declare(default)]
  pub indeterminate: bool,
}

pub const UNCHECKED_ICON: &str = "unchecked_box";
pub const CHECKED_ICON: &str = "checked_box";
pub const INDETERMINATE_ICON: &str = "indeterminate_box";

#[derive(Template)]
pub enum CheckboxChild {
  Label(TextInit),
  Before(Leading<TextInit>),
  After(Trailing<TextInit>),
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

  fn icon_name(&self) -> &'static str {
    if self.indeterminate {
      INDETERMINATE_ICON
    } else if self.checked {
      CHECKED_ICON
    } else {
      UNCHECKED_ICON
    }
  }
}

impl ComposeChild<'static> for Checkbox {
  type Child = Option<CheckboxChild>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    rdl! {
      let checkbox = @Class {
        class: distinct_pipe!($this.state_class_name()),
        @Icon {
          class: CHECKBOX,
          cursor: CursorIcon::Pointer,
          @pipe!(named_svgs::get($this.icon_name()))
        }
      };
      let checkbox = if let Some(child) = child {
        let row = @Row { align_items: Align::Center };
        let row = match child {
          CheckboxChild::Label(text) | CheckboxChild::Before(Leading(text)) => @ $row {
            @ { checkbox }
            @Text { text }
          },
          CheckboxChild::After(Trailing(text)) => @ $row {
            @Text { text}
            @ { checkbox }
          },
        };
        FatObj::new(row.into_widget())
      } else {
        checkbox.map(|w| w.into_widget())
      };

      @ $checkbox {
        on_tap: move |_| $this.write().switch_check(),
        on_key_up: move |k| if *k.key() == VirtualKey::Named(NamedKey::Space) {
          $this.write().switch_check()
        }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
pub mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_image_tests!(
    checkbox,
    WidgetTester::new(self::column! {
      @Checkbox { checked: true, @ { "checked" } }
      @Checkbox { indeterminate: true, @Leading("indeterminate") }
      @Checkbox { @Trailing("unchecked") }
    })
    .with_wnd_size(Size::new(240., 160.)),
  );
}
