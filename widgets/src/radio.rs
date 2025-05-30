use ribir_core::prelude::*;

use crate::prelude::*;

class_names! {
  /// The class name for the radio button when selected
  RADIO_SELECTED,
  /// The class name for the radio button when unselected
  RADIO_UNSELECTED,
  /// Basic class name for the radio button
  RADIO,
  /// The icon class name for the selected radio button
  RADIO_SELECTED_ICON,
  /// The icon class name for the unselected radio button
  RADIO_UNSELECTED_ICON
}

/// A radio button allows you to select one option from a group of options.
///
/// It also supports associating a label with the radio button, with the label
/// inheriting the text style from its nearest ancestor. The label can be
/// positioned before or after the radio button using the `Leading` and
/// `Trailing` types, with the default position being after the radio button.
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _radio = radio! { selected: true, value: 1, };
/// ```
///
/// It also supports placing a label before or after the radio button.
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let _radio = radio! {
///   @ { "Default label placed after the radio button!" }
/// };
/// let _leading = radio! {
///   @Leading::new("Leading label placed before the radio button!")
/// };
/// let _trailing = radio! {
///   @Trailing::new("Trailing label placed after the radio button!")
/// };
/// ```
#[derive(Declare)]
pub struct Radio {
  #[declare(default)]
  pub selected: bool,
  #[declare(custom, default = Box::new(()) as Box<dyn Any>)]
  pub value: Box<dyn Any>,
}

impl RadioDeclarer {
  /// Initialize the radio value without supporting the pipe value format.
  pub fn with_value<V: 'static>(&mut self, value: V) -> &mut Self {
    self.value = Some(PipeValue::Value(Box::new(value)));
    self
  }
}

impl Radio {
  fn radio_class_name(&self) -> ClassName {
    if self.selected { RADIO_SELECTED } else { RADIO_UNSELECTED }
  }

  fn radio_icon_class_name(&self) -> ClassName {
    if self.selected { RADIO_SELECTED_ICON } else { RADIO_UNSELECTED_ICON }
  }
}

impl ComposeChild<'static> for Radio {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fat_obj! {
      on_tap: move |_| $this.write().selected = true,
      @ {
        let classes = class_array![distinct_pipe!($this.radio_class_name()), RADIO];
        let icon = @ $classes {
          @Icon {
            on_key_up: move |k| if *k.key() == VirtualKey::Named(NamedKey::Space) {
              $this.write().selected = true
            },
            @Void { class: distinct_pipe!($this.radio_icon_class_name()) }
          }
        };
        icon_with_label(icon.into_widget(), child)
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, test_helper::*};
  use ribir_dev_helper::*;

  use crate::prelude::*;

  widget_image_tests!(
    radio_widget,
    WidgetTester::new(self::column! {
      @Radio { selected: true }
      @Radio {
        selected: false,
        @ { "Default label position." }
      }
      @Radio {
        selected: true,
        @Leading::new("Leading label position.")
      }
      @Radio {
        selected: false,
        @Trailing::new("Trailing label position.")
      }
    })
    .with_wnd_size(Size::new(200., 256.))
    .with_comparison(0.002)
  );
}
