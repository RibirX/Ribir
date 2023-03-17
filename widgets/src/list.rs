use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare)]
pub struct Lists;

#[derive(Clone)]
pub struct ListsStyle {
  pub padding: EdgeInsets,
}

impl CustomTheme for ListsStyle {}

#[derive(Declare)]
pub struct ListsDecorator {}
impl ComposeStyle for ListsDecorator {
  type Host = Widget;

  #[inline]
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

impl ComposeChild for Lists {
  type Child = Vec<Widget>;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    widget! {
      init ctx => {
        let ListsStyle { padding } = ListsStyle::of(ctx).clone();
        let surface: Brush = Palette::of(ctx).surface().clone().into();
      }
      ListsDecorator {
        background: surface.clone(),
        Column {
          padding,
          DynWidget {
            dyns: child.into_iter().map(|w| {
              widget! {
                ListItemDecorator {
                  DynWidget::from(w)
                }
              }
            })
          }
        }
      }
    }
  }
}

#[derive(Declare, Default)]
pub struct ListItem;

#[derive(Clone)]
pub struct ListItemStyle {
  pub height: f32,
  pub padding: EdgeInsets,
  pub label_gap: EdgeInsets,
  pub leading_gap: EdgeInsets,
  pub trailing_gap: EdgeInsets,
  pub headline_style: CowArc<TextStyle>,
  pub supporting_style: CowArc<TextStyle>,
}

impl CustomTheme for ListItemStyle {}

#[derive(Declare)]
pub struct ListItemDecorator {}

impl ComposeStyle for ListItemDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

pub struct HeadlineText(pub Label);
pub struct SupportingText(pub Label);

#[derive(Template)]
pub struct ListItemTemplate {
  headline: State<HeadlineText>,
  supporting: Option<State<SupportingText>>,
  leading: Option<WidgetOf<Leading>>,
  trailing: Option<WidgetOf<Trailing>>,
}

impl ComposeChild for ListItem {
  type Child = ListItemTemplate;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    let ListItemTemplate {
      headline,
      supporting,
      leading,
      trailing,
    } = child;

    widget! {
      states { headline: headline.into_readonly() }
      init ctx => {
        let ListItemStyle {
          height,
          padding,
          label_gap,
          leading_gap,
          trailing_gap,
          headline_style,
          supporting_style,
        } = ListItemStyle::of(ctx).clone();
        let on_surface: Brush = Palette::of(ctx)
          .on_surface()
          .clone()
          .into();
        let on_surface_variant: Brush = Palette::of(ctx)
          .on_surface_variant()
          .clone()
          .into();
      }

      ConstrainedBox {
        padding,
        clamp: BoxClamp::fixed_height(height),
        Column {
          Row {
            Option::map(leading, |w| widget! {
              DynWidget {
                padding: leading_gap,
                dyns: w.child,
              }
            })
            Expanded {
              padding: label_gap,
              flex: 1.,
              Column {
                Text::new(
                  headline.0.0.clone(),
                  &on_surface,
                  headline_style.clone()
                )
                Option::map(supporting, |supporting| widget! {
                  states { supporting: supporting.into_readonly() }
                  Text::new(
                    supporting.0.0.clone(),
                    &on_surface_variant,
                    supporting_style.clone(),
                  )
                })
              }
            }
            Option::map(trailing, |w| widget! {
              DynWidget {
                padding: trailing_gap,
                dyns: w.child,
              }
            })
          }
        }
      }
    }
  }
}
