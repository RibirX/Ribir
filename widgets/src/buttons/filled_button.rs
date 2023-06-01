use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};
use ribir_core::prelude::*;

#[derive(Clone)]
pub struct FilledButtonStyle {
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub radius: f32,
  pub padding_style: EdgeInsets,
}

impl CustomStyle for FilledButtonStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    FilledButtonStyle {
      height: 40.,
      icon_size: Size::splat(18.),
      label_gap: 8.,
      icon_pos: IconPosition::Before,
      label_style: TypographyTheme::of(ctx).label_large.text.clone(),
      radius: 20.,
      padding_style: EdgeInsets::horizontal(16.),
    }
  }
}

#[derive(Clone, Declare)]
pub struct FilledButtonDecorator {
  #[allow(unused)]
  pub button_type: ButtonType,
  pub color: Color,
}

impl ComposeDecorator for FilledButtonDecorator {
  type Host = Widget;

  fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
}

/// FilledButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{FilledButton, Label};
///
/// // only icon
/// let filled_icon_button = widget! {
///   FilledButton { svgs::ADD }
/// };
///
/// // only label
/// let filled_label_button = widget! {
///   FilledButton { Label::new("filled button") }
/// };
///
/// // both icon and label
/// let filled_button = widget! {
///    FilledButton {
///     svgs::ADD
///     Label::new("filled button")
///   }
/// };
///
/// // use custom color
/// let custom_color_button = widget! {
///   FilledButton {
///     color: Color::RED,
///     svgs::ADD
///     Label::new("filled button")
///   }
/// };
/// ```
#[derive(Declare, Default)]
pub struct FilledButton {
  #[declare(default=Palette::of(ctx).primary())]
  color: Color,
}

impl ComposeChild for FilledButton {
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
        let FilledButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
        } = FilledButtonStyle::of(ctx).clone();
        let palette1 = Palette::of(ctx).clone();
        let palette2 = Palette::of(ctx).clone();
      }
      FilledButtonDecorator {
        button_type,
        color: this.color,
        ButtonImpl {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          background_color: Brush::from(palette1.base_of(&this.color)),
          foreground_color: Brush::from(palette2.on_of(&palette2.base_of(&this.color))),
          radius,
          border_style: None,
          padding_style,

          widget::from(child)
        }
      }
    }
  }
}
