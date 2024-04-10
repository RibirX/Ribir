use ribir_core::prelude::*;

use super::{ButtonImpl, ButtonTemplate, ButtonType, IconPosition};

#[derive(Clone)]
pub struct ButtonStyle {
  pub height: f32,
  pub icon_size: Size,
  pub label_gap: f32,
  pub icon_pos: IconPosition,
  pub label_style: CowArc<TextStyle>,
  pub padding_style: EdgeInsets,
}

impl CustomStyle for ButtonStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    ButtonStyle {
      height: 40.,
      icon_size: Size::splat(18.),
      label_gap: 8.,
      icon_pos: IconPosition::Before,
      label_style: TypographyTheme::of(ctx).label_large.text.clone(),
      padding_style: EdgeInsets::horizontal(16.),
    }
  }
}

#[derive(Clone, Declare)]
pub struct ButtonDecorator {
  #[allow(unused)]
  pub button_type: ButtonType,
  pub color: Color,
}

impl ComposeDecorator for ButtonDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

/// Button usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{Button, Label};
///
/// // only icon
/// let raw_icon_button = fn_widget! {
///   @Button { @{ svgs::ADD } }
/// };
///
/// // only label
/// let raw_label_button = fn_widget! {
///   @Button { @ { Label::new("raw button") } }
/// };
///
/// // use custom color
/// let custom_color_button = fn_widget! {
///   @Button {
///     color: Color::RED,
///     @{ Label::new("raw button") }
///   }
/// };
/// ```
#[derive(Default, Declare)]
pub struct Button {
  #[declare(default=Palette::of(ctx!()).primary())]
  color: Color,
}

impl ComposeChild for Button {
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
        let ButtonStyle {
          height,
          icon_size,
          label_gap,
          icon_pos,
          label_style,
          padding_style,
        } = ButtonStyle::of(ctx!());
        let palette = Palette::of(ctx!()).clone();

        @ButtonDecorator {
          button_type,
          color: pipe!($this.color),
          @ButtonImpl {
            height,
            icon_size,
            label_gap,
            icon_pos,
            label_style,
            background_color: None,
            foreground_color: pipe!(Brush::from(palette.base_of(&$this.color))),
            radius: None,
            border_style: None,
            padding_style,

            @ { child }
          }
        }
      }
    }
  }
}
