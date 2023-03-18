use crate::prelude::*;
use ribir_core::prelude::*;

/// Lists usage
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let w = widget! {
///   Lists {
///     ListItem {
///     Leading {
///       Icon { svgs::ADD_CIRCLE }
///     }
///     HeadlineText(Label::new("One line list item"))
///     SupportingText(Label::new("One line supporting text"))
///     Trailing {
///       Icon { svgs::CHECK_BOX_OUTLINE_BLANK }
///     }
///   }
///   Divider { indent: 16. }
///   ListItem {
///     item_align: Align::Start,
///     Leading {
///       Icon { svgs::HOME }
///     }
///     HeadlineText(Label::new("More line list item"))
///     SupportingText(Label::new("More line supporting text \r more lines supporting text \r more lines supporting text"))
///     Trailing {
///       Text { text: "100+" }
///     }
///   }
///  }
/// };
/// ```
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
          DynWidget { dyns: child.into_iter() }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct ListItemImpl {
  #[declare(convert=strip_option)]
  pub height: Option<f32>,
  pub padding_style: EdgeInsets,
  pub label_gap: EdgeInsets,
  pub trailing_gap: EdgeInsets,
  #[declare(default=Align::Center)]
  pub item_align: Align,
  #[declare(default=TypographyTheme::of(ctx).body_large.text.clone())]
  pub headline_style: CowArc<TextStyle>,
  #[declare(default=TypographyTheme::of(ctx).body_medium.text.clone())]
  pub supporting_style: CowArc<TextStyle>,
}

#[derive(Declare)]
pub struct ListItemImplDecorator {}

impl ComposeStyle for ListItemImplDecorator {
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

impl ComposeChild for ListItemImpl {
  type Child = ListItemTemplate;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let ListItemTemplate {
      headline,
      supporting,
      leading,
      trailing,
    } = child;

    widget! {
      states {
        this: this.into_readonly(),
        headline: headline.into_readonly(),
      }
      init ctx => {
        let palette = Palette::of(ctx);
        let on_surface: Brush = palette.on_surface().clone().into();
        let on_surface_variant: Brush = palette.on_surface_variant().clone().into();
      }
      ListItemImplDecorator {
        DynWidget {
          dyns: this.height.map(|height| ConstrainedBox {
            clamp: BoxClamp::fixed_height(height)
          }),
          Padding {
            padding: this.padding_style,
            Column {
              Row {
                align_items: this.item_align,
                Option::map(leading, |w| w.child)
                Expanded {
                  flex: 1.,
                  Column {
                    padding: this.label_gap,
                    Text {
                      text: headline.0.0.clone(),
                      foreground: on_surface,
                      style: this.headline_style.clone(),
                    }
                    Option::map(supporting, |supporting| widget! {
                      states { supporting: supporting.into_readonly() }
                      Text {
                        text: supporting.0.0.clone(),
                        foreground: on_surface_variant,
                        style: this.supporting_style.clone(),
                      }
                    })
                  }
                }
                Option::map(trailing, |w| widget! {
                  DynWidget {
                    padding: this.trailing_gap,
                    dyns: w.child
                  }
                })
              }
            }
          }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct ListItem {
  #[declare(default=Align::Center)]
  pub item_align: Align,
}

#[derive(Clone)]
pub struct ListItemStyle {
  pub padding_style: EdgeInsets,
  pub label_gap: EdgeInsets,
  pub trailing_gap: EdgeInsets,
}

impl CustomTheme for ListItemStyle {}

#[derive(Clone, Declare)]
pub struct ListItemDecorator {}

impl ComposeStyle for ListItemDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

impl ComposeChild for ListItem {
  type Child = ListItemTemplate;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let ListItemStyle {
          padding_style,
          label_gap,
          trailing_gap,
        } = ListItemStyle::of(ctx).clone();
      }
      ListItemDecorator {
        ListItemImpl {
          height: None,
          padding_style,
          label_gap,
          trailing_gap,
          item_align: this.item_align,
          DynWidget::from(child)
        }
      }
    }
  }
}
