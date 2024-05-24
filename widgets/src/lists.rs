use ribir_core::prelude::*;

use crate::prelude::*;

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
/// fn_widget! {
///   @Lists {
///     @ListItem {
///       @{ HeadlineText(Label::new("One line list item")) }
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
/// fn_widget! {
///   @Lists {
///     @ListItem {
///       @ { HeadlineText(Label::new("headline text")) }
///       @ { SupportingText(Label::new("supporting text")) }
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
/// fn_widget! {
///   @Lists {
///     // use leading icon
///     @ListItem {
///       @Leading { @{ svgs::CHECK_BOX_OUTLINE_BLANK } }
///       @HeadlineText(Label::new("headline text"))
///     }
///     // use leading label
///     @ListItem {
///       @Leading { @{ Label::new("A") } }
///       @HeadlineText(Label::new("headline text"))
///     }
///     // use leading custom widget
///     @ListItem {
///       @Leading {
///         @CustomEdgeWidget(
///           @Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }.build(ctx!())
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
/// fn_widget! {
///   @Lists {
///     // use trailing icon
///     @ListItem {
///       @HeadlineText(Label::new("headline text"))
///       @Trailing { @{ svgs::CHECK_BOX_OUTLINE_BLANK } }
///     }
///     // use trailing label
///     @ListItem {
///       @HeadlineText(Label::new("headline text"))
///       @Trailing { @{ Label::new("A") } }
///     }
///     // use trailing custom widget
///     @ListItem {
///       @HeadlineText(Label::new("headline text"))
///       @Trailing {
///         @CustomEdgeWidget(
///           @Container {
///             size: Size::splat(40.),
///             background: Color::YELLOW,
///           }.build(ctx!())
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
/// fn_widget! {
///   @Lists {
///     @ListItem {
///       @ { HeadlineText(Label::new("One line list item")) }
///     }
///     @Divider {}
///     @ListItem {
///       @ { HeadlineText(Label::new("One line list item")) }
///     }
///   }
/// };
/// ```
#[derive(Declare)]
pub struct Lists;

#[derive(Declare)]
pub struct ListsDecorator {}
impl ComposeDecorator for ListsDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

impl ComposeChild for Lists {
  type Child = BoxPipe<Vec<Widget>>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @ListsDecorator {
        @Column { @ { child.into_pipe() } }
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

pub struct Poster(pub Resource<PixelImage>);

pub struct HeadlineText(pub Label);
pub struct SupportingText(pub Label);

#[derive(Template)]
pub enum EdgeWidget {
  Text(State<Label>),
  Icon(NamedSvg),
  Avatar(FatObj<Pair<State<Avatar>, AvatarTemplate>>),
  Image(Resource<PixelImage>),
  Poster(Poster),
  Custom(CustomEdgeWidget),
}

pub struct CustomEdgeWidget(pub Widget);

impl EdgeWidget {
  fn compose_with_style(self, config: EdgeWidgetStyle) -> impl WidgetBuilder {
    let EdgeWidgetStyle { icon, text, avatar, image, poster, custom } = config;
    fn_widget! {
      let w: Widget = match self {
        EdgeWidget::Icon(w) => {
          let margin = icon.gap.map(|margin| Margin { margin });
          @ $margin {
            @Icon {
              size: icon.size,
              @ { w }
            }
          }
        },
        EdgeWidget::Text(label) => {
          let margin =  text.gap.map(|margin| Margin { margin });
          @ $margin {
            @Text {
              text: $label.0.clone(),
              text_style: text.style.clone(),
              foreground: text.foreground.clone(),
            }
          }
        },
        EdgeWidget::Avatar(w) => {
          let margin = avatar.gap.map(|margin| Margin { margin });
          @ $margin { @ { w }}
        },
        EdgeWidget::Image(w) => {
          let margin = image.gap.map(|margin| Margin { margin });
          @ $margin {
            @SizedBox {
              size: image.size,
              @ $w { box_fit: BoxFit::None }
            }
          }
        },
        EdgeWidget::Poster(w) => {
          let margin = poster.gap.map(|margin| Margin { margin });
          let w = w.0;
          @ $margin {
            @ SizedBox {
              size: poster.size,
              @ $w { box_fit: BoxFit::None }
            }
          }
        },
        EdgeWidget::Custom(w) => {
          let margin = custom.gap.map(|margin| Margin { margin });
          @$margin { @ { w.0 }}
        },
      };
      w
    }
  }
}

#[derive(Template)]
pub struct ListItemTml {
  headline: State<HeadlineText>,
  supporting: Option<State<SupportingText>>,
  leading: Option<Pair<FatObj<Leading>, EdgeWidget>>,
  trailing: Option<Pair<FatObj<Trailing>, EdgeWidget>>,
}

impl ComposeChild for ListItem {
  type Child = ListItemTml;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    let ListItemTml { headline, supporting, leading, trailing } = child;

    fn_widget! {
      let ListItemStyle {
        padding_style,
        label_gap,
        headline_style,
        supporting_style,
        leading_config,
        trailing_config,
        item_align,
      } = ListItemStyle::of(ctx!());

      let padding = padding_style.map(|padding| Padding { padding });
      let label_gap = label_gap.map(|padding| Padding { padding });

      @ListItemDecorator {
        color: pipe!($this.active_background),
        is_active: false,
        @ $padding {
          @Row {
            align_items: pipe!(item_align($this.line_number)),
            @{
              leading.map(move |w| {
                let (leading, widget) = w.unzip();
                leading.map(|_| widget.compose_with_style(leading_config))
              })
            }
            @Expanded {
              flex: 1.,
              @ $label_gap {
                @Column {
                  @Text {
                    text: pipe!($headline.0.0.clone()),
                    foreground: Palette::of(ctx!()).on_surface(),
                    text_style: headline_style,
                  }
                  @{ supporting.map(|supporting|  {
                    @ConstrainedBox {
                      clamp: {
                        let TextStyle { line_height, font_size, .. } = &*supporting_style;
                        let line_height = line_height.map_or(*font_size, FontSize::Em).into_pixel();
                        pipe!{
                          let text_height = line_height * $this.line_number as f32;
                          BoxClamp::fixed_height(*text_height)
                        }
                      } ,
                      @Text {
                        text: pipe!($supporting.0.0.clone()),
                        foreground:  Palette::of(ctx!()).on_surface_variant(),
                        text_style: supporting_style,
                      }
                    }
                  })}
                }
              }
            }
            @{ trailing.map(|w| {
              let (trailing, widget) = w.unzip();
              trailing.map(|_| widget.compose_with_style(trailing_config))
            })}
          }
        }
      }
    }
  }
}

#[derive(Declare)]
pub struct ListItem {
  #[declare(default = 1usize)]
  pub line_number: usize,
  #[declare(default = Palette::of(ctx!()).primary())]
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
      padding_style: Some(EdgeInsets { left: 0., right: 24., bottom: 8., top: 8. }),
      item_align: |num| {
        if num >= 2 { Align::Start } else { Align::Center }
      },
      label_gap: Some(EdgeInsets::only_left(16.)),
      headline_style: typography.body_large.text.clone(),
      supporting_style: typography.body_medium.text.clone(),
      leading_config: EdgeWidgetStyle {
        icon: EdgeItemStyle { size: Size::splat(24.), gap: Some(EdgeInsets::only_left(16.)) },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle { size: Size::splat(40.), gap: Some(EdgeInsets::only_left(16.)) },
        image: EdgeItemStyle { size: Size::splat(56.), gap: Some(EdgeInsets::only_left(16.)) },
        poster: EdgeItemStyle { size: Size::new(120., 64.), gap: None },
        custom: EdgeItemStyle { size: Size::splat(40.), gap: Some(EdgeInsets::only_left(16.)) },
      },
      trailing_config: EdgeWidgetStyle {
        icon: EdgeItemStyle { size: Size::splat(24.), gap: Some(EdgeInsets::only_left(16.)) },
        text: EdgeTextItemStyle {
          style: typography.label_small.text.clone(),
          foreground: palette.on_surface_variant().into(),
          gap: Some(EdgeInsets::only_left(16.)),
        },
        avatar: EdgeItemStyle { size: Size::splat(40.), gap: Some(EdgeInsets::only_left(16.)) },
        image: EdgeItemStyle { size: Size::splat(56.), gap: Some(EdgeInsets::only_left(16.)) },
        poster: EdgeItemStyle { size: Size::new(120., 64.), gap: None },
        custom: EdgeItemStyle { size: Size::splat(40.), gap: Some(EdgeInsets::only_left(16.)) },
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
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder {
    fn_widget! { host }
  }
}
