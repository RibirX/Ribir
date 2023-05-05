use crate::prelude::*;
use ribir_core::prelude::*;

/// Avatar usage
///
/// # Example
///
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Avatar {
///     Label::new("A")
///   }
/// };
///
/// # #[cfg(feature="png")]
/// widget! {
///   Avatar {
///     ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"))
///   }
/// };
/// ```
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

impl CustomStyle for AvatarStyle {}

pub struct AvatarDecorator;

impl ComposeDecorator for AvatarDecorator {
  type Host = Widget;

  fn compose_decorator(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

#[derive(Template)]
pub enum AvatarTemplate {
  Text(State<Label>),
  Image(ShallowImage),
}

impl ComposeChild for Avatar {
  type Child = AvatarTemplate;
  type Target = Widget;
  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    widget! {
      init ctx => {
        let AvatarStyle {
          size, radius, background, text_style, text_color,
        } = AvatarStyle::of(ctx).clone();
      }
      SizedBox {
        size,
        widget::from(match child {
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
          }.into_widget(),
          AvatarTemplate::Image(image) => widget! {
            DynWidget {
              dyns: radius.map(|radius| {
                let path = Path::rect_round(
                  &Rect::from_size(size),
                  &Radius::all(radius),
                  PathStyle::Fill,
                );
                Clip { clip: ClipType::Path(path) }
              }),
              Container {
                size,
                widget::from(image)
              }
            }
          }.into_widget()
        })
      }
    }
    .into_widget()
  }
}

pub(crate) fn add_to_theme(theme: &mut FullTheme) {
  theme.custom_styles.set_custom_style(AvatarStyle {
    size: Size::splat(40.),
    radius: Some(20.),
    background: Some(theme.palette.primary().into()),
    text_color: theme.palette.on_primary().into(),
    text_style: theme.typography_theme.body_large.text.clone(),
  });
}
