use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};
use ribir_core::prelude::*;

#[derive(Clone)]
pub struct OutlinedButtonStyle {
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub radius: f32,
  pub padding_style: EdgeInsets,
  pub border_width: f32,
}

impl CustomTheme for OutlinedButtonStyle {}

#[derive(Clone, Declare)]
pub struct OutlinedButtonDecorator {
  #[allow(unused)]
  button_type: ButtonType,
}

impl ComposeStyle for OutlinedButtonDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

/// OutlinedButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // only icon
/// let outlined_icon_button = widget! {
///   OutlinedButton { svgs::ADD }
/// };
///
/// // only label
/// let outlined_label_button = widget! {
///   OutlinedButton { Label::new("outlined button") }
/// };
///
/// // both icon and label
/// let outlined_button = widget! {
///    OutlinedButton {
///     svgs::ADD
///     Label::new("outlined button")
///   }
/// };
///
/// // use custom color
/// let custom_color_button = widget! {
///   OutlinedButton {
///     color: Brush::Color(Color::RED),
///     svgs::ADD
///     Label::new("outlined button")
///   }
/// };
/// ```
#[derive(Declare, Default)]
pub struct OutlinedButton {
  #[declare(default=Palette::of(ctx).primary(), convert=into)]
  color: Brush,
}

impl ComposeChild for OutlinedButton {
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
        let OutlinedButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
          border_width,
        } = OutlinedButtonStyle::of(ctx).clone();
        let palette1 = Palette::of(ctx).clone();
        let palette2 = Palette::of(ctx).clone();
      }
      OutlinedButtonDecorator {
        button_type,
        ButtonImpl {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          background_color: None,
          foreground_color: this.color
            .only_convert_color(|color| palette1.base_of(color)),
          radius,
          border_style: Border::all(BorderSide {
            width: border_width,
            color: this.color.only_convert_color(|color| palette2.base_of(color)),
          }),
          padding_style,

          DynWidget::from(child)
        }
      }
    }
  }
}
