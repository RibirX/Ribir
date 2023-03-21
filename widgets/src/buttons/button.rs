use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};
use ribir_core::prelude::*;

#[derive(Clone)]
pub struct RawButtonStyle {
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub padding_style: EdgeInsets,
}

impl CustomTheme for RawButtonStyle {}

#[derive(Clone, Declare)]
pub struct RawButtonDecorator {
  #[allow(unused)]
  button_type: ButtonType,
}

impl ComposeStyle for RawButtonDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

/// Button usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{Button, Label, svgs};
///
/// // only icon
/// let raw_icon_button = widget! {
///   Button { svgs::ADD }
/// };
///
/// // only label
/// let raw_label_button = widget! {
///   Button { Label::new("raw button") }
/// };
///
/// // use custom color
/// let custom_color_button = widget! {
///   Button {
///     color: Color::RED,
///     Label::new("raw button")
///   }
/// };
/// ```
#[derive(Declare, Default)]
pub struct Button {
  #[declare(default=Palette::of(ctx).primary(), convert=into)]
  color: Brush,
}

impl ComposeChild for Button {
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
        let RawButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          padding_style,
        } = RawButtonStyle::of(ctx).clone();
        let palette = Palette::of(ctx).clone();
      }
      RawButtonDecorator {
        button_type,
        ButtonImpl {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          background_color: None,
          foreground_color: this.color
            .only_convert_color(|color| palette.base_of(color)),
          radius: None,
          border_style: None,
          padding_style,

          DynWidget::from(child)
        }
      }
    }
  }
}
