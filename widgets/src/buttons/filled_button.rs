use ribir_core::prelude::*;

use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};

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
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

/// FilledButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{FilledButton, Label};
///
/// // only icon
/// let filled_icon_button = fn_widget! {
///   @FilledButton { @{ svgs::ADD } }
/// };
///
/// // only label
/// let filled_label_button = fn_widget! {
///   @FilledButton { @{ Label::new("filled button") } }
/// };
///
/// // both icon and label
/// let filled_button = fn_widget! {
///    @FilledButton {
///     @ { svgs::ADD }
///     @ { Label::new("filled button") }
///   }
/// };
///
/// // use custom color
/// let custom_color_button = fn_widget! {
///   @FilledButton {
///     color: Color::RED,
///     @ { svgs::ADD }
///     @ { Label::new("filled button") }
///   }
/// };
/// ```
#[derive(Declare, Default)]
pub struct FilledButton {
  #[declare(default=Palette::of(ctx!()).primary())]
  pub color: Color,
}

impl ComposeChild for FilledButton {
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
        let FilledButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
        } = FilledButtonStyle::of(ctx!());

        @FilledButtonDecorator {
          button_type,
          color: pipe!($this.color),
          @ButtonImpl {
            height,
            icon_size,
            label_gap,
            icon_pos,
            label_style,
            background_color: pipe!(Palette::of(ctx!()).base_of(&$this.color)),
            foreground_color: pipe! {
              let palette = Palette::of(ctx!());
              palette.on_of(&palette.base_of(&$this.color))
            },
            radius,
            border_style: None,
            padding_style,

            @ { child }
          }
        }
      }
    }
  }
}
