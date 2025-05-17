//! Buttons enable users to take action and make choices with a single tap.
//!
//! We provide four types of buttons for you to use: [`Button`],
//! [`FilledButton`], [`OutlinedButton`], [`Fab`], [`MiniFab`], and
//! [`LargeFab`].
//!
//! Each button has a distinct style and emphasized behavior.
//!
//! Both of them can accept two optional children:
//! - A string as a label.
//! - And a widget as an icon.
//!
//! ## Usage
//!
//! ```
//! # use ribir_core::prelude::*;
//! # use ribir_widgets::prelude::*;
//!
//! // button with label only
//! let _ = button! {
//!   @ { "Label only" }
//! };
//!
//! // button with icon only
//! let _ = button! {
//!   @Icon { @SpinnerProgress {} }
//! };
//!
//! // button with both label and icon
//! let _ = button! {
//!   @Icon { @Icon { @SpinnerProgress {} } }
//!   @ { "Label" }
//! };
//! ```
//!
//! By default, the icon will be placed on the leading side of the label. You
//! can also use [`Leading`] and [`Trailing`] to explicitly set the position of
//! the icon.
//!
//! ```
//! # use ribir_core::prelude::*;
//! # use ribir_widgets::prelude::*;
//!
//! let _leading = button! {
//!   @Leading::new(@Icon { @SpinnerProgress {} })
//!   @ { "Leading" }
//! };
//!
//! let _trailing = button! {
//!   @Trailing::new(@Icon { @SpinnerProgress {} })
//!   @ { "Trailing" }
//! };
//! ```
//!
//! ## Changing the Color
//!
//! The theme should adapt to the `Color` provider to change the button's color.
//! If you opt for a custom color, I suggest adjusting the color's lightness
//! tone using the palette to ensure suitability for both light and dark themes.
//!
//! ```
//! # use ribir_core::prelude::*;
//! # use ribir_widgets::prelude::*;
//!
//! let _ = fn_widget! {
//!   let p = Palette::of(BuildCtx::get());
//!   let dyn_color = Stateful::new(p.base_of(&Color::YELLOW));
//!   @Row {
//!     @Button {
//!       providers: [Provider::new(p.tertiary())],
//!       @ { "Tertiary" }
//!     }
//!     @Button {
//!       providers: [Provider::new(p.base_of(&Color::RED))],
//!       @ { "Custom Color" }
//!     }
//!     @Button {
//!       providers: [Provider::value_of_writer(dyn_color, None)],
//!       @ { "Dynamic Color" }
//!     }
//!   }
//! };
//! ```
use ribir_core::prelude::*;

use crate::prelude::*;

/// Represents the default button, usually with a border.
///
/// See the [module-level documentation](self) for more.
#[derive(Default, Declare)]
pub struct Button;

/// Represents a button with a filled background.
///
/// See the [module-level documentation](self) for more.
#[derive(Declare, Default)]
pub struct FilledButton;

/// Represents a text button without a border or background, suitable for
/// low-emphasis actions.
///
/// See the [module-level documentation](self) for more.
#[derive(Default, Declare)]
pub struct TextButton;

/// Represents a floating action button that typically floats at the bottom of
/// the screen.
///
/// See the [module-level documentation](self) for base usage.
///
/// The `Fab` theme incorporates the `FabSize` provider to define its size and
/// shape, allowing you to customize the button's dimensions. However, it is
/// important to note that the `FabSize` is assumed to be read-only, meaning
/// that the theme will not adjust to changes even if you provide a
/// `Stateful<FabSize>`.
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _ = fab! {
///  providers: [Provider::new(FabSize::Large)],
///  @Icon { @SpinnerProgress {} }
///  @ { "Label" }
/// };
/// ```
#[derive(Default, Declare)]
pub struct Fab;

#[derive(Debug, Clone, Copy)]
pub enum FabSize {
  Mini,
  Normal,
  Large,
}

/// The template child for buttons indicating the possible label and
/// icon types the button can have.
#[derive(Template)]
pub struct ButtonChild<'c> {
  label: Option<TextValue>,
  icon: Option<PositionChild<Widget<'c>>>,
}

class_names! {
  #[doc = "This class specifies a fully basic button, including both an icon and a label."]
  BUTTON,
  #[doc="This class specifies for the label of the basic button."]
  BTN_LABEL,
  #[doc="This class specifies for the leading icon of the basic button."]
  BTN_LEADING_ICON,
  #[doc="This class specifies for the trailing icon of the basic button."]
  BTN_TRAILING_ICON,
  #[doc="This class specifies for the icon-only basic button."]
  BTN_ICON_ONLY,
  #[doc="This class specifies for the label-only basic button."]
  BTN_LABEL_ONLY,
}

impl<'c> ComposeChild<'c> for Button {
  type Child = ButtonChild<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    child.compose_to_widget([
      BUTTON,
      BTN_LEADING_ICON,
      BTN_TRAILING_ICON,
      BTN_LABEL,
      BTN_ICON_ONLY,
      BTN_LABEL_ONLY,
    ])
  }
}

class_names! {
  #[doc = "This class specifies a fully text button, including both an icon and a label."]
  TEXT_BTN,
  #[doc = "This class specifies for the label of the text button."]
  TEXT_BTN_LABEL,
  #[doc = "This class specifies for the leading icon of the text button."]
  TEXT_BTN_LEADING_ICON,
  #[doc = "This class specifies for the trailing icon of the text button."]
  TEXT_BTN_TRAILING_ICON,
  #[doc = "This class specifies for the icon-only text button."]
  TEXT_BTN_ICON_ONLY,
  #[doc = "This class specifies for the label-only text button."]
  TEXT_BTN_LABEL_ONLY,
}

impl<'c> ComposeChild<'c> for TextButton {
  type Child = ButtonChild<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    child.compose_to_widget([
      TEXT_BTN,
      TEXT_BTN_LEADING_ICON,
      TEXT_BTN_TRAILING_ICON,
      TEXT_BTN_LABEL,
      TEXT_BTN_ICON_ONLY,
      TEXT_BTN_LABEL_ONLY,
    ])
  }
}

class_names! {
  #[doc = "This class specifies a fully filled button, including both an icon and a label."]
  FILLED_BTN,
  #[doc = "This class specifies for the label of the filled button."]
  FILLED_BTN_LABEL,
  #[doc = "This class specifies for the leading icon of the filled button."]
  FILLED_BTN_LEADING_ICON,
  #[doc = "This class specifies for the trailing icon of the filled button."]
  FILLED_BTN_TRAILING_ICON,
  #[doc = "This class specifies for the icon-only filled button."]
  FILLED_BTN_ICON_ONLY,
  #[doc = "This class specifies for the label-only filled button."]
  FILLED_BTN_LABEL_ONLY,
}

impl<'c> ComposeChild<'c> for FilledButton {
  type Child = ButtonChild<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    child.compose_to_widget([
      FILLED_BTN,
      FILLED_BTN_LEADING_ICON,
      FILLED_BTN_TRAILING_ICON,
      FILLED_BTN_LABEL,
      FILLED_BTN_ICON_ONLY,
      FILLED_BTN_LABEL_ONLY,
    ])
  }
}

class_names! {
  #[doc = "This class specifies a fully fab button, including both an icon and a label."]
  FAB,
  #[doc = "This class specifies for the label of the fab button."]
  FAB_LABEL,
  #[doc = "This class specifies for the leading icon of the fab button."]
  FAB_LEADING_ICON,
  #[doc = "This class specifies for the trailing icon of the fab button."]
  FAB_TRAILING_ICON,
  #[doc = "This class specifies for the icon-only fab button."]
  FAB_ICON_ONLY,
  #[doc = "This class specifies for the label-only fab button."]
  FAB_LABEL_ONLY
}

impl<'c> ComposeChild<'c> for Fab {
  type Child = ButtonChild<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    child.compose_to_widget([
      FAB,
      FAB_LEADING_ICON,
      FAB_TRAILING_ICON,
      FAB_LABEL,
      FAB_ICON_ONLY,
      FAB_LABEL_ONLY,
    ])
  }
}

impl<'c> ButtonChild<'c> {
  /// Convert the button child into a widget by a horizontal layout and assign
  /// the specified class name to it.
  ///
  /// - If there is no label or icon, the button will be empty, and the `btn`
  ///   class will be assigned.
  /// - If only an icon is present, the `icon_only` class will be assigned to
  ///   the icon.
  /// - If only a label is present, the `label_only` class will be assigned to
  ///   the label.
  /// - If both an icon and a label are present, the `btn` class will be
  ///   assigned to the button, the `btn_icon` class will be assigned to the
  ///   icon, and the `btn_label` class will be assigned to the label.
  fn compose_to_widget(
    self,
    [btn, btn_leading_icon, btn_trialing_icon, btn_label, icon_only, label_only]: [ClassName; 6],
  ) -> Widget<'c> {
    let Self { label, icon } = self;
    match (label, icon) {
      (None, None) => void!( class: btn ).into_widget(),
      (None, Some(icon)) => fat_obj! {
        class: icon_only,
        @ { icon.unwrap() }
      }
      .into_widget(),
      (Some(text), None) => text! { class: label_only, text }.into_widget(),
      (Some(text), Some(icon)) => rdl! {
        let trailing_icon = icon.is_trailing();
        let icon = @Class {
          class: if trailing_icon { btn_trialing_icon } else { btn_leading_icon },
          @ { icon.unwrap() }
        }.into_widget();

        let label = @Text { class: btn_label, text }.into_widget();
        let items = if trailing_icon { [label, icon] } else { [icon, label] };

        @Row {
          class: btn,
          align_items: Align::Center,
          justify_content: JustifyContent::Center,
          @ { items }
        }
      }
      .into_widget(),
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  fn miss_icon() -> Svg { named_svgs::get_or_default("default") }

  widget_image_tests!(
    button,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      line_gap: 20.,
      wrap: true,
      // icon only
      @TextButton { @Icon { @miss_icon() } }
      // label only
      @TextButton { @{ "Label only"} }
      @TextButton {
        @Icon { @miss_icon() }
        @ { "Default icon position" }
      }
      @TextButton {
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      @TextButton {
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(400., 120.)),
  );

  widget_image_tests!(
    filled_button,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      v_align: Align::Center,
      line_gap: 20.,
      wrap: true,
      // icon only
      @FilledButton { @Icon { @miss_icon() } }
      // label only
      @FilledButton { @{ "Label only"} }
      @FilledButton {
        @Icon { @miss_icon() }
        @ { "Default icon position" }
      }
      @FilledButton {
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      @FilledButton {
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(400., 128.))
    .with_comparison(0.00004),
  );

  widget_image_tests!(
    outlined_button,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      v_align: Align::Center,
      line_gap: 20.,
      wrap: true,
      // icon only
      @Button { @Icon { @miss_icon() } }
      // label only
      @Button { @{ "Label only"} }
      @Button {
        @Icon { @miss_icon() }
        @ { "Default icon position" }
      }
      @Button {
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      @Button {
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(400., 128.)),
  );

  widget_image_tests!(
    mini_fab,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      v_align: Align::Center,
      line_gap: 20.,
      wrap: true,
      // icon only
      @Fab {
        providers: [Provider::new(FabSize::Mini)],
        @Icon { @miss_icon() }
      }
      // label only
      @Fab {
        providers: [Provider::new(FabSize::Mini)],
        @{ "Label only"}
      }
      @Fab {
        providers: [Provider::new(FabSize::Mini)],
        @Icon { @miss_icon() }
        @ { "Default icon position"
      }
      }
      @Fab {
        providers: [Provider::new(FabSize::Mini)],
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      @Fab {
        providers: [Provider::new(FabSize::Mini)],
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(400., 128.)),
  );

  widget_image_tests!(
    fab,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      v_align: Align::Center,
      line_gap: 20.,
      wrap: true,
      // icon only
      @Fab { @Icon { @miss_icon() } }
      // label only
      @Fab { @{ "Label only"} }
      @Fab {
        @Icon { @miss_icon() }
        @ { "Default icon position" }
      }
      @Fab {
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      @Fab {
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(400., 164.)),
  );

  widget_image_tests!(
    large_fab,
    WidgetTester::new(flex! {
      justify_content: JustifyContent::SpaceAround,
      v_align: Align::Center,
      line_gap: 20.,
      wrap: true,
      // label only
      @Fab {
        providers: [Provider::new(FabSize::Large)],
        @{ "Label only"}
      }
      @Fab {
        providers: [Provider::new(FabSize::Large)],
        @Icon { @miss_icon() }
        @ { "Default icon position" }
      }
      @Fab {
        providers: [Provider::new(FabSize::Large)],
        @Leading::new(@Icon { @miss_icon() })
        @ { "Leading icon" }
      }
      // icon only
      @Fab {
        providers: [Provider::new(FabSize::Large)],
        @Icon { @miss_icon() }
      }
      @Fab {
        providers: [Provider::new(FabSize::Large)],
        @ { "Trailing icon" }
        @Trailing::new(@Icon { @miss_icon() })
      }
    })
    .with_wnd_size(Size::new(640., 256.)),
  );
}
