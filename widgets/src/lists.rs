use crate::prelude::*;
use ribir_core::prelude::*;

/// Lists usage
///
/// use `ListItem` must have `HeadlineText`, other like `SupportingText`,
/// `Leading`, and `Trailing` are optional.
///
/// # Example
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
///         widget! {
///           Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }
///         }
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
///         widget! {
///           Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }
///         }
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

#[derive(Declare)]
pub struct ListsDecorator {}
impl ComposeDecorator for ListsDecorator {
  type Host = Widget;

  fn compose_decorator(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

impl ComposeChild for Lists {
  type Child = Vec<Widget>;

  fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
    widget! {
      ListsDecorator {
        Column {
          DynWidget { dyns: child.into_iter() }
        }
      }
    }
  }
}

#[derive(Clone, Default)]
pub struct EdgeItemStyle {
  pub size: Size,
  pub gap: Option<EdgeInsets>,
}

#[derive(Clone)]
pub struct EdgeTextItemStyle {
  pub style: CowArc<TextStyle>,
  pub gap: Option<EdgeInsets>,
  pub foreground: Brush,
}

#[derive(Clone)]
pub struct EdgeWidgetStyle {
  pub icon: EdgeItemStyle,
  pub text: EdgeTextItemStyle,
  pub avatar: EdgeItemStyle,
  pub image: EdgeItemStyle,
  pub poster: EdgeItemStyle,
  pub custom: EdgeItemStyle,
}

pub struct Poster(pub ShareResource<PixelImage>);

pub struct HeadlineText(pub Label);
pub struct SupportingText(pub Label);

impl TmlFlag for Leading {}
impl TmlFlag for Trailing {}

#[derive(Template)]
pub enum EdgeWidget<P: TmlFlag + Default + 'static> {
  Text(DecorateTml<P, State<Label>>),
  Icon(DecorateTml<P, NamedSvg>),
  Avatar(DecorateTml<P, ComposePair<State<Avatar>, AvatarTemplate>>),
  Image(DecorateTml<P, ShareResource<PixelImage>>),
  Poster(DecorateTml<P, Poster>),
  Custom(DecorateTml<P, Widget>),
}

impl<P> EdgeWidget<P>
where
  P: TmlFlag + Default,
{
  fn compose_with_style(self, config: EdgeWidgetStyle) -> Widget {
    let EdgeWidgetStyle {
      icon,
      text,
      avatar,
      image,
      poster,
      custom,
    } = config;
    match self {
      EdgeWidget::Icon(w) => widget! {
        DynWidget {
          dyns: icon.gap.map(|margin| Margin { margin }),
          Icon {
            size: icon.size,
            widget::from(w.decorate(|_, c| c))
          }
        }
      },
      EdgeWidget::Text(w) => widget! {
        DynWidget {
          dyns: text.gap.map(|margin| Margin { margin }),
          widget::from(w.decorate(|_, label| widget!{
            states { label: label.into_readonly() }
            Text {
              text: label.0.clone(),
              text_style: text.style.clone(),
              foreground: text.foreground.clone(),
            }
          }))
        }
      },
      EdgeWidget::Avatar(w) => widget! {
        DynWidget {
          dyns: avatar.gap.map(|margin| Margin { margin }),
          DecorateTml::decorate(w, |_, c| c.into_widget())
        }
      },
      EdgeWidget::Image(w) => widget! {
        DynWidget {
          dyns: image.gap.map(|margin| Margin { margin }),
          SizedBox {
            size: image.size,
            DynWidget {
              box_fit: BoxFit::None,
              dyns: w.decorate(|_, c| c)
            }
          }
        }
      },
      EdgeWidget::Poster(w) => widget! {
        DynWidget {
          dyns: poster.gap.map(|margin| Margin { margin }),
          SizedBox {
            size: poster.size,
            DynWidget {
              box_fit: BoxFit::None,
              dyns: w.decorate(|_, c| c.0)
            }
          }
        }
      },
      EdgeWidget::Custom(w) => widget! {
        DynWidget {
          dyns: custom.gap.map(|margin| Margin { margin }),
          widget::from(w.decorate(|_, c| c))
        }
      },
    }
  }
}

#[derive(Template)]
pub struct ListItemTemplate {
  headline: State<HeadlineText>,
  supporting: Option<State<SupportingText>>,
  #[template(flat_fill)]
  leading: Option<EdgeWidget<Leading>>,
  #[template(flat_fill)]
  trailing: Option<EdgeWidget<Trailing>>,
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
        color: this.active_background,
        is_active: false,
        DynWidget {
          dyns: padding_style.map(|padding| Padding { padding }),
          Row {
            align_items: item_align(this.line_number),
            Option::map(leading, |w| w.compose_with_style(leading_config))
            Expanded {
              flex: 1.,
              DynWidget {
                dyns: label_gap.map(|padding| Padding { padding }),
                Clip {
                  Column {
                    Text {
                      text: headline.0.0.clone(),
                      foreground: on_surface,
                      text_style: headline_style.clone(),
                    }
                    Option::map(supporting, |supporting| widget! {
                      states { supporting: supporting.into_readonly() }
                      ConstrainedBox {
                        clamp: BoxClamp::fixed_height(*text_height.0),
                        Text {
                          text: supporting.0.0.clone(),
                          foreground: on_surface_variant.clone(),
                          text_style: supporting_style.clone(),
                        }
                      }
                    })
                  }
                }
              }
            }
            Option::map(trailing, |w| w.compose_with_style(trailing_config))
          }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct ListItem {
  #[declare(default = 1)]
  pub line_number: usize,
  #[declare(default = Palette::of(ctx).primary())]
  pub active_background: Color,
}

#[derive(Clone)]
pub struct ListItemStyle {
  pub padding_style: Option<EdgeInsets>,
  pub label_gap: Option<EdgeInsets>,
  pub item_align: fn(usize) -> Align,
  pub headline_style: CowArc<TextStyle>,
  pub supporting_style: CowArc<TextStyle>,
  pub leading_config: EdgeWidgetStyle,
  pub trailing_config: EdgeWidgetStyle,
}

impl CustomStyle for ListItemStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    let typography = TypographyTheme::of(ctx);
    let palette = Palette::of(ctx);
    ListItemStyle {
      padding_style: Some(EdgeInsets {
        left: 0.,
        right: 24.,
        bottom: 8.,
        top: 8.,
      }),
      item_align: |num| {
        if num >= 2 {
          Align::Start
        } else {
          Align::Center
        }
      },
      label_gap: Some(EdgeInsets::only_left(16.)),
      headline_style: typography.body_large.text.clone(),
      supporting_style: typography.body_medium.text.clone(),
      leading_config: EdgeWidgetStyle {
        icon: EdgeItemStyle {
          size: Size::splat(24.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        image: EdgeItemStyle {
          size: Size::splat(56.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        poster: EdgeItemStyle {
          size: Size::new(120., 64.),
          gap: None,
        },
        custom: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
      },
      trailing_config: EdgeWidgetStyle {
        icon: EdgeItemStyle {
          size: Size::splat(24.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        image: EdgeItemStyle {
          size: Size::splat(56.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        poster: EdgeItemStyle {
          size: Size::new(120., 64.),
          gap: None,
        },
        custom: EdgeItemStyle {
          size: Size::splat(40.),
          gap: Some(EdgeInsets::only_left(16.)),
        },
      },
    }
  }
}

#[derive(Clone, Declare)]
pub struct ListItemDecorator {
  pub color: Color,
  pub is_active: bool,
}

impl ComposeDecorator for ListItemDecorator {
  type Host = Widget;
  fn compose_decorator(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}
