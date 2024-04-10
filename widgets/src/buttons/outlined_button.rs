use ribir_core::prelude::*;

use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};

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

impl CustomStyle for OutlinedButtonStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    OutlinedButtonStyle {
      height: 40.,
      icon_size: Size::splat(18.),
      label_gap: 8.,
      icon_pos: IconPosition::Before,
      label_style: TypographyTheme::of(ctx).label_large.text.clone(),
      radius: 20.,
      padding_style: EdgeInsets::horizontal(16.),
      border_width: 1.,
    }
  }
}

#[derive(Clone, Declare)]
pub struct OutlinedButtonDecorator {
  #[allow(unused)]
  pub button_type: ButtonType,
  pub color: Color,
}

impl ComposeDecorator for OutlinedButtonDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

/// OutlinedButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{OutlinedButton, Label};
///
/// // only icon
/// let outlined_icon_button = fn_widget! {
///   @OutlinedButton { @{ svgs::ADD } }
/// };
///
/// // only label
/// let outlined_label_button = fn_widget! {
///   @OutlinedButton { @{ Label::new("outlined button") } }
/// };
///
/// // both icon and label
/// let outlined_button = fn_widget! {
///    @OutlinedButton {
///     @ { svgs::ADD }
///     @ { Label::new("outlined button")}
///   }
/// };
///
/// // use custom color
/// let custom_color_button = fn_widget! {
///   @OutlinedButton {
///     color: Color::RED,
///     @ { svgs::ADD }
///     @ { Label::new("outlined button") }
///   }
/// };
/// ```
#[derive(Default, Declare)]
pub struct OutlinedButton {
  #[declare(default=Palette::of(ctx!()).primary())]
  color: Color,
}

impl ComposeChild for OutlinedButton {
  type Child = ButtonTemplate;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    let ButtonTemplate { icon, label } = &child;
    let button_type = match (&icon, &label) {
      (Some(_), Some(_)) => ButtonType::BOTH,
      (Some(_), None) => ButtonType::ICON,
      (None, Some(_)) => ButtonType::LABEL,
      (None, None) => panic!("Button content cannot be empty!"),
    };

    fn_widget! {
      @ {
        let OutlinedButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
          border_width,
        } = OutlinedButtonStyle::of(ctx!());

        @OutlinedButtonDecorator {
          button_type,
          color: pipe!($this.color),
          @ButtonImpl {
            height,
            icon_size,
            label_gap,
            icon_pos,
            label_style,
            background_color: None,
            foreground_color: pipe!(Palette::of(ctx!()).base_of(&$this.color)),
            radius,
            border_style: pipe!(Border::all(BorderSide {
              width: border_width,
              color: Palette::of(ctx!()).base_of(&$this.color).into()
            })),
            padding_style,

            @ { child }
          }
        }
      }
    }
  }
}
