use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, Default)]
pub struct Button;

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonTheme {
  /// The button padding value
  pub padding: f32,
  /// The button radius value
  pub radius: f32,
  /// The button border style
  pub border_color: Color,
  /// The button background color
  pub background: Color,
  /// The button foreground color
  pub foreground: Color,
}

pub struct ButtonText(pub CowArc<str>);

impl ButtonText {
  pub fn new(v: impl Into<CowArc<str>>) -> Self { ButtonText(v.into()) }
}

#[derive(Template)]
pub struct ButtonTemplate {
  button_text: State<ButtonText>,
  icon: Option<WidgetOf<Leading>>,
}

impl ComposeChild for Button {
  type Child = ButtonTemplate;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    let ButtonTemplate { icon, button_text } = child;

    widget! {
      init ctx => {
        let ButtonTheme {
          padding,
          radius,
          background,
          foreground,
          // border_color,
          ..
        } = *ButtonTheme::of(ctx);
        let style = &TypographyTheme::of(ctx).label_large.text;
      }

      states {
        button_text: button_text.into_readonly(),
      }
      Row {
        padding: EdgeInsets::all(padding),
        border_radius: Radius::all(radius),
        // todo: border and background render has little gap?
        // border: Border::all(BorderSide { width: 1., color: border_color }),
        background,
        justify_content: JustifyContent::Center,

        DynWidget {
          dyns: icon.map(|w| w.child)
        }
        Text {
          text: button_text.0.clone(),
          foreground: foreground.into(),
          style: style.clone(),
        }
      }
    }
  }
}

impl CustomTheme for ButtonTheme {}
