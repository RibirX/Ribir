use crate::prelude::*;
use ribir_core::prelude::*;

/// Lists usage
///
/// use `ListItem` must have `HeadlineText`, other like `SupportingText`,
/// `Leading`, and `Trailing` are optional.
///
/// # example
///
/// ## single headline text
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // only single headline text
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///   }
/// };
/// ```
///
/// ## headline text and supporting text
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // single headline text and supporting text
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       SupportingText(Label::new("supporting text"))
///     }
///   }
/// };
/// ```
///
/// ## use leading
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     // use leading icon
///     ListItem {
///       Leading { svgs::CHECK_BOX_OUTLINE_BLANK }
///       HeadlineText(Label::new("headline text"))
///     }
///     // use leading label
///     ListItem {
///       Leading { Label::new("A") }
///       HeadlineText(Label::new("headline text"))
///     }
///     // use leading custom widget
///     ListItem {
///       Leading {
///         IntoWidget::into_widget(
///           widget! {
///             Container {
///               size: Size::splat(40.),
///               background: Color::YELLOW,
///             }
///           }
///         )
///       }
///     }
///   }
/// };
/// ```
///
/// ## use trailing
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     // use trailing icon
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing { svgs::CHECK_BOX_OUTLINE_BLANK }
///     }
///     // use trailing label
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing { Label::new("A") }
///     }
///     // use trailing custom widget
///     ListItem {
///       HeadlineText(Label::new("headline text"))
///       Trailing {
///         IntoWidget::into_widget(
///           widget! {
///             Container {
///               size: Size::splat(40.),
///               background: Color::YELLOW,
///             }
///           }
///         )
///       }
///     }
///   }
/// };
/// ```
///
/// ## use `Divider` split list item
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// widget! {
///   Lists {
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///     Divider {}
///     ListItem {
///       HeadlineText(Label::new("One line list item"))
///     }
///   }
/// };
/// ```
#[derive(Declare)]
pub struct Lists;

#[derive(Clone)]
pub struct ListsStyle {
  pub padding: EdgeInsets,
  pub background: Brush,
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
        let ListsStyle { padding, background, } = ListsStyle::of(ctx).clone();
      }
      ListsDecorator {
        background,
        Column {
          padding,
          DynWidget { dyns: child.into_iter() }
        }
      }
    }
  }
}

#[derive(Clone, Default)]
pub struct ItemInfo {
  pub size: Size,
  pub gap: Option<EdgeInsets>,
}

#[derive(Clone)]
pub struct TextItemInfo {
  pub style: CowArc<TextStyle>,
  pub gap: Option<EdgeInsets>,
  pub foreground: Brush,
}

#[derive(Clone)]
pub struct ListItemConfig {
  pub icon: ItemInfo,
  pub text: TextItemInfo,
  pub avatar: ItemInfo,
  // pub image: ItemInfo,
  // pub poster: ItemInfo,
  pub custom: ItemInfo,
}

pub struct HeadlineText(pub Label);
pub struct SupportingText(pub Label);

#[derive(Template)]
pub enum LeadingWidget {
  Text(DecorateTml<Leading, State<Label>>),
  Icon(DecorateTml<Leading, NamedSvg>),
  Avatar(DecorateTml<Leading, ComposePair<State<Avatar>, AvatarTemplate>>),
  // Todo: Image,
  // Todo: Poster,
  Custom(DecorateTml<Leading, Widget>),
}

impl TmlFlag for Leading {}
impl TmlFlag for Trailing {}

#[derive(Template)]
pub enum TrailingWidget {
  Text(DecorateTml<Trailing, State<Label>>),
  Icon(DecorateTml<Trailing, NamedSvg>),
  Avatar(DecorateTml<Trailing, ComposePair<State<Avatar>, AvatarTemplate>>),
  // Todo: Image,
  // Todo: Poster,
  Custom(DecorateTml<Trailing, Widget>),
}

#[derive(Template)]
pub struct ListItemTemplate {
  headline: State<HeadlineText>,
  supporting: Option<State<SupportingText>>,
  #[template(flat_fill)]
  leading: Option<LeadingWidget>,
  #[template(flat_fill)]
  trailing: Option<TrailingWidget>,
}

impl ComposeChild for ListItem {
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
        let ListItemStyle {
          padding_style,
          label_gap,
          headline_style,
          supporting_style,
          leading_config,
          trailing_config,
          item_align,
        } = ListItemStyle::of(ctx).clone();
        let TextStyle { line_height, font_size, .. } = *supporting_style.clone();
        let line_height = line_height
          .map_or(font_size, FontSize::Em)
          .into_pixel();
        let text_height = line_height * this.line_number as f32;
      }
      ListItemDecorator {
        DynWidget {
          dyns: padding_style.map(|padding| Padding { padding }),
          Row {
            align_items: item_align(this.line_number),
            Option::map(leading, |leading| {
              let ListItemConfig {
                icon,
                text,
                avatar,
                custom,
              } = leading_config.clone();
              match leading {
                LeadingWidget::Icon(w) => widget! {
                  DynWidget {
                    dyns: icon.gap.map(|margin| Margin { margin }),
                    Icon {
                      size: icon.size,
                      widget::from(w.decorate(|_, c| c))
                    }
                  }
                },
                LeadingWidget::Text(w) => widget! {
                  DynWidget {
                    dyns: text.gap.map(|margin| Margin { margin }),
                    widget::from(w.decorate(|_, label| widget!{
                      states { label: label.into_readonly() }
                      Text {
                        text: label.0.clone(),
                        style: text.style.clone(),
                        foreground: text.foreground.clone(),
                      }
                    }))
                  }
                },
                LeadingWidget::Avatar(w) => widget! {
                  DynWidget {
                    dyns: avatar.gap.map(|margin| Margin { margin }),
                    SizedBox {
                      size: avatar.size,
                      DynWidget {
                        box_fit: BoxFit::Contain,
                        dyns: w.decorate(|_, c| c.into_widget())
                      }
                    }
                  }
                },
                LeadingWidget::Custom(w) => widget! {
                  DynWidget {
                    dyns: custom.gap.map(|margin| Margin { margin }),
                    SizedBox {
                      size: custom.size,
                      DynWidget {
                        box_fit: BoxFit::Contain,
                        dyns: w.decorate(|_, c| c)
                      }
                    }
                  }
                },
              }
            })
            Expanded {
              flex: 1.,
              DynWidget {
                dyns: label_gap.map(|padding| Padding { padding }),
                Column {
                  Text {
                    text: headline.0.0.clone(),
                    foreground: on_surface,
                    style: headline_style.clone(),
                  }
                  Option::map(supporting, |supporting| widget! {
                    states { supporting: supporting.into_readonly() }
                    ConstrainedBox {
                      clamp: BoxClamp::fixed_height(*text_height.0),
                      Text {
                        text: supporting.0.0.clone(),
                        foreground: on_surface_variant.clone(),
                        style: supporting_style.clone(),
                      }
                    }
                  })
                }
              }
            }
            Option::map(trailing, |trailing| {
              let ListItemConfig {
                icon,
                text,
                avatar,
                custom,
              } = trailing_config.clone();
              match trailing {
                TrailingWidget::Icon(w) => widget! {
                  DynWidget {
                    dyns: icon.gap.map(|margin| Margin { margin }),
                    Icon {
                      size: icon.size,
                      widget::from(w.decorate(|_, c| c))
                    }
                  }
                },
                TrailingWidget::Text(w) => widget! {
                  DynWidget {
                    dyns: text.gap.map(|margin| Margin { margin }),
                    widget::from(w.decorate(|_, label| widget!{
                      states { label: label.into_readonly() }
                      Text {
                        text: label.0.clone(),
                        style: text.style.clone(),
                        foreground: text.foreground.clone(),
                      }
                    }))
                  }
                },
                TrailingWidget::Avatar(w) => widget! {
                  DynWidget {
                    dyns: avatar.gap.map(|margin| Margin { margin }),
                    SizedBox {
                      size: avatar.size,
                      DynWidget {
                        box_fit: BoxFit::Contain,
                        dyns: w.decorate(|_, c| c.into_widget())
                      }
                    }
                  }
                },
                TrailingWidget::Custom(w) => widget! {
                  DynWidget {
                    dyns: custom.gap.map(|margin| Margin { margin }),
                    SizedBox {
                      size: custom.size,
                      DynWidget {
                        box_fit: BoxFit::Contain,
                        dyns: w.decorate(|_,c| c)
                      }
                    }
                  }
                }
              }
            })
          }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct ListItem {
  #[declare(default = 0)]
  pub line_number: usize,
}

#[derive(Clone)]
pub struct ListItemStyle {
  pub padding_style: Option<EdgeInsets>,
  pub label_gap: Option<EdgeInsets>,
  pub item_align: fn(usize) -> Align,
  pub headline_style: CowArc<TextStyle>,
  pub supporting_style: CowArc<TextStyle>,
  pub leading_config: ListItemConfig,
  pub trailing_config: ListItemConfig,
}

impl CustomTheme for ListItemStyle {}

#[derive(Clone, Declare)]
pub struct ListItemDecorator {}

impl ComposeStyle for ListItemDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}
