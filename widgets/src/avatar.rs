use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Avatar;

#[derive(Clone)]
pub struct AvatarStyle {
  pub size: Size,
  pub radius: Option<f32>,
  pub background: Option<Brush>,
  pub text_color: Brush,
  pub text_style: CowArc<TextStyle>,
}

impl CustomTheme for AvatarStyle {}

pub struct AvatarDecorator;

impl ComposeStyle for AvatarDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

#[derive(Template)]
pub enum AvatarTemplate {
  Text(State<Label>),
  // Image(Image),
}

impl ComposeChild for Avatar {
  type Child = AvatarTemplate;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    widget! {
      init ctx => {
        let AvatarStyle {
          size, radius, background, text_style, text_color,
        } = AvatarStyle::of(ctx).clone();
      }
      SizedBox {
        size,
        DynWidget {
          dyns: match child {
            AvatarTemplate::Text(text) => widget! {
              states { text: text.into_readonly() }
              BoxDecoration {
                background,
                border_radius: radius.map(Radius::all),
                Container {
                  size,
                  Text {
                    h_align: HAlign::Center,
                    v_align: VAlign::Center,
                    text: text.0.clone(),
                    style: text_style.clone(),
                    foreground: text_color.clone(),
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
