use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, Default)]
pub struct Lists {
  #[declare(default = false)]
  divider: bool,
}

#[derive(Clone, PartialEq)]
pub enum EdgePosition {
  Frist,
  Last,
  None,
}

#[derive(Clone, Declare)]
pub struct ListItemStyle {
  #[declare(default = false)]
  pub divider: bool,
  pub edge: EdgePosition,
}

impl ComposeStyle for ListItemStyle {
  type Host = Widget;
  #[inline]
  fn compose_style(_this: Stateful<Self>, host: Self::Host) -> Widget
  where
    Self: Sized,
  {
    host
  }
}

pub struct HeadlineText(pub CowArc<str>);

impl HeadlineText {
  pub fn new(v: impl Into<CowArc<str>>) -> Self { HeadlineText(v.into()) }
}

pub struct SupportingText(pub CowArc<str>);

impl SupportingText {
  pub fn new(v: impl Into<CowArc<str>>) -> Self { SupportingText(v.into()) }
}

#[derive(Default, Declare)]
pub struct ListItem;

#[derive(Template)]
pub struct ListItemTemplate {
  headline_text: HeadlineText,
  supporting_text: Option<SupportingText>,
  leading: Option<WidgetOf<Leading>>,
  trailing: Option<WidgetOf<Trailing>>,
}

impl ComposeChild for ListItem {
  type Child = ListItemTemplate;

  fn compose_child(_this: State<Self>, child: Self::Child) -> Widget {
    let ListItemTemplate {
      headline_text,
      supporting_text,
      leading,
      trailing,
    } = child;

    widget! {
      init ctx => {
        let surface_variant = Brush::Color(Palette::of(ctx).on_surface_variant());
        let body_text_style = TypographyTheme::of(ctx).body1.text.clone();
      }
      Row {
        Option::map(leading, |w| w.child)
        Expanded {
          flex: 1.,
          Column {
            Text {
              text: headline_text.0,
              style: TextStyle {
                foreground: surface_variant,
                ..body_text_style
              }
            }
            Option::map(supporting_text, |text | widget! { Text { text: text.0 } })
          }
        }
        Option::map(trailing, |w| w.child)
      }
    }
  }
}

impl ComposeChild for Lists {
  type Child = Vec<Widget>;

  fn compose_child(this: State<Self>, children: Self::Child) -> Widget {
    let last_idx = children.len() - 1;

    widget! {
      states {
        this: this.into_readonly()
      }
      Column {
        DynWidget {
          dyns: children.into_iter().enumerate().map(move |(idx, w)| {
            let edge = if idx == 0 {
              EdgePosition::Frist
            } else if idx == last_idx {
              EdgePosition::Last
            } else {
              EdgePosition::None
            };

            widget! {
              ListItemStyle {
                divider: this.divider,
                edge,
                identify(w)
              }
            }
          })
        }
      }
    }
  }
}
