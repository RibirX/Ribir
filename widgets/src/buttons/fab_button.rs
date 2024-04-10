use ribir_core::prelude::*;

use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};

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

impl CustomStyle for FabButtonStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    FabButtonStyle {
      height: 56.,
      icon_size: Size::splat(24.),
      label_gap: 8.,
      icon_pos: IconPosition::Before,
      label_style: TypographyTheme::of(ctx).label_large.text.clone(),
      radius: 16.,
      padding_style: EdgeInsets::horizontal(16.),
    }
  }
}

#[derive(Clone, Declare)]
pub struct FabButtonDecorator {
  #[allow(unused)]
  pub button_type: ButtonType,
  pub color: Color,
}

impl ComposeDecorator for FabButtonDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

/// FabButton usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{FabButton, Label};
///
/// // only icon
/// let fab_icon_button = fn_widget! {
///   @FabButton { @{ svgs::ADD } }
/// };
///
/// // only label
/// let fab_label_button = fn_widget! {
///   @FabButton { @ { Label::new("fab button") } }
/// };
///
/// // both icon and label
/// let fab_button = fn_widget! {
///    @FabButton {
///     @ { svgs::ADD }
///     @ { Label::new("fab button") }
///   }
/// };
///
/// // use custom color
/// let custom_color_button = fn_widget! {
///   @FabButton {
///     color: Color::RED,
///     @ { svgs::ADD }
///     @ { Label::new("fab button") }
///   }
/// };
/// ```
#[derive(Default, Declare)]
pub struct FabButton {
  #[declare(default=Palette::of(ctx!()).primary())]
  color: Color,
}

impl ComposeChild for FabButton {
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
        let FabButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          radius,
          padding_style,
        } = FabButtonStyle::of(ctx!());
        let palette1 = Palette::of(ctx!()).clone();
        let palette2 = Palette::of(ctx!()).clone();

        @FabButtonDecorator {
          button_type,
          color: pipe!($this.color),
          @ButtonImpl {
            height,
            icon_size,
            label_gap,
            icon_pos,
            label_style,
            background_color: pipe!(Brush::from(palette1.base_of(&$this.color))),
            foreground_color: pipe!(Brush::from(palette2.on_of(&palette2.base_of(&$this.color)))),
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
