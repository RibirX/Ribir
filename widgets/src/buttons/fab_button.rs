use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};
use ribir_core::prelude::*;

#[derive(Clone)]
pub struct FabButtonStyle {
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub radius: f32,
  pub padding_style: EdgeInsets,
}

impl CustomTheme for FabButtonStyle {}

#[derive(Clone, Declare)]
pub struct FabButtonDecorator {
  #[allow(unused)]
  button_type: ButtonType,
}

impl ComposeStyle for FabButtonDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

/// FabButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{FabButton, Label, svgs};
///
/// // only icon
/// let fab_icon_button = widget! {
///   FabButton { svgs::ADD }
/// };
///
/// // only label
/// let fab_label_button = widget! {
///   FabButton { Label::new("fab button") }
/// };
///
/// // both icon and label
/// let fab_button = widget! {
///    FabButton {
///     svgs::ADD
///     Label::new("fab button")
///   }
/// };
///
/// // use custom color
/// let custom_color_button = widget! {
///   FabButton {
///     color: Color::RED,
///     svgs::ADD
///     Label::new("fab button")
///   }
/// };
/// ```
#[derive(Declare, Default)]
pub struct FabButton {
  #[declare(default=Palette::of(ctx).primary(), convert=into)]
  color: Brush,
}

impl ComposeChild for FabButton {
  type Child = ButtonTemplate;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let ButtonTemplate { icon, label } = &child;
    let button_type = match (&icon, &label) {
      (Some(_), Some(_)) => ButtonType::BOTH,
      (Some(_), None) => ButtonType::ICON,
      (None, Some(_)) => ButtonType::LABEL,
      (None, None) => panic!("Button content cannot be empty!"),
    };

    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let FabButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
        } = FabButtonStyle::of(ctx).clone();
        let palette1 = Palette::of(ctx).clone();
        let palette2 = Palette::of(ctx).clone();
      }
      FabButtonDecorator {
        button_type,
        ButtonImpl {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          background_color: this.color
            .only_convert_color(|color| palette1.base_of(color)),
          foreground_color: this.color
            .only_convert_color(|color| palette2.on_of(&palette2.base_of(color))),
          radius,
          border_style: None,
          padding_style,

          widget::from(child)
        }
      }
    }
  }
}
