use ribir_core::prelude::*;

use crate::prelude::*;

/// Tabs usage
///
/// # Example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let tabs = fn_widget! {
///   @Tabs {
///     @Tab {
///       @TabItem {
///         @ { svgs::HOME }
///         @ { Label::new("Home") }
///       }
///       @TabPane {
///         @{ fn_widget!{ @Text { text: "content" } } }
///       }
///     }
///     @Tab {
///       @TabItem {
///         @ { svgs::HOME }
///         @ { Label::new("Home") }
///       }
///       @TabPane {
///         @{ fn_widget!{ @Text { text: "content" } } }
///       }
///     }
///   }
/// };
///
/// // bottom tabs
/// let bottom_tabs = fn_widget! {
///   @Tabs {
///     pos: Position::Bottom,
///     @Tab {
///       @TabItem {
///         @ { svgs::HOME }
///         @ { Label::new("Home") }
///       }
///       @TabPane {
///         @{ fn_widget!{ @Text { text: "content" } } }
///       }
///     }
///     @Tab {
///       @TabItem {
///         @ { svgs::HOME }
///         @ { Label::new("Home") }
///       }
///       @TabPane {
///         @{ fn_widget!{ @Text { text: "content" } } }
///       }
///     }
///   }
/// };
/// ```
#[derive(Declare, Clone)]
pub struct Tabs {
  #[declare(default = Position::Top)]
  pub pos: Position,
  #[declare(default)]
  pub cur_idx: usize,
}

#[derive(Clone)]
pub struct IndicatorStyle {
  pub measure: Option<f32>,
  pub extent: f32,
}

#[derive(Clone)]
pub struct TabsStyle {
  pub extent_only_label: f32,
  pub extent_only_icon: f32,
  pub extent_with_both: f32,
  pub icon_size: Size,
  pub icon_pos: Position,
  pub active_color: Brush,
  pub foreground: Brush,
  pub label_style: CowArc<TextStyle>,
  pub indicator: IndicatorStyle,
}

impl CustomStyle for TabsStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    let palette = Palette::of(ctx);
    TabsStyle {
      extent_with_both: 64.,
      extent_only_label: 48.,
      extent_only_icon: 48.,
      icon_size: Size::splat(24.),
      icon_pos: Position::Top,
      active_color: palette.primary().into(),
      foreground: palette.on_surface_variant().into(),
      label_style: TypographyTheme::of(ctx).title_small.text.clone(),
      indicator: IndicatorStyle { extent: 3., measure: Some(60.) },
    }
  }
}
#[derive(Declare)]
pub struct TabsDecorator {}

impl ComposeDecorator for TabsDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

#[derive(Template)]
pub struct Tab {
  label: TabItem,
  child: Pair<TabPane, GenWidget>,
}

#[derive(Template)]
pub struct TabItem {
  icon: Option<NamedSvg>,
  text: Option<State<Label>>,
}

#[derive(PairChild)]
#[simple_declare]
pub struct TabPane;

#[derive(Declare)]
pub struct TabDecorator {}

impl ComposeDecorator for TabDecorator {
  fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
}

#[derive(Declare)]
pub struct IndicatorDecorator {
  pub pos: Position,
  pub rect: Rect,
  pub extent: f32,
}

impl ComposeDecorator for IndicatorDecorator {
  fn compose_decorator(this: State<Self>, host: Widget) -> impl WidgetBuilder {
    fn_widget! {
      @ $host{
        anchor: pipe!{
          let this = $this;
          let x = match this.pos {
            Position::Top | Position::Bottom =>
              this.rect.origin.x + (this.rect.size.width - 60.) / 2.,
            Position::Left => this.rect.size.width - this.extent,
            Position::Right => 0.,
          };
          let y = match this.pos {
            Position::Left | Position::Right => this.rect.origin.y
              + (this.rect.size.height - 60.) / 2.,
            Position::Top => this.rect.size.height - this.extent,
            Position::Bottom => 0.,
          };
          Anchor::left_top(x, y)
        },
      }
    }
  }
}

impl Tabs {
  fn tab_header(
    headers: Vec<(Option<NamedSvg>, Option<State<Label>>)>, tabs_style: TabsStyle,
    tabs: impl StateWriter<Value = Tabs> + 'static,
    indicator: impl StateWriter<Value = IndicatorDecorator> + 'static,
  ) -> impl Iterator<Item = impl WidgetBuilder> {
    let TabsStyle { icon_size: size, icon_pos, active_color, foreground, label_style, .. } =
      tabs_style;
    headers
      .into_iter()
      .enumerate()
      .map(move |(idx, (icon, label))| {
        let tabs = tabs.clone_writer();
        let active_color = active_color.clone();
        let foreground = foreground.clone();
        let label_style = label_style.clone();
        let indicator = indicator.clone_writer();
        fn_widget! {
          let icon_widget = icon.map(|icon| @Icon { size, @ { icon }});
          let label_widget = label.map(|label| {
            @Text {
              text: pipe!($label.0.clone()),
              foreground: pipe!(match $tabs.cur_idx == idx {
                true => active_color.clone(),
                false => foreground.clone(),
              }),
              text_style: label_style,
            }
          });
          @ {
            let mut tab_header = @Expanded {
              on_tap: move |_| if $tabs.cur_idx != idx {
                $tabs.write().cur_idx = idx;
              },
            };

            let u = watch!(($tabs.cur_idx == idx, $tab_header.layout_rect()))
              .filter_map(|(active, rect)| active.then_some(rect))
              .subscribe(move |v| $indicator.write().rect = v);

            @TabDecorator {
              on_disposed: move |_| { u.unsubscribe(); },
              @$tab_header {
                @Flex {
                  align_items: Align::Center,
                  justify_content: JustifyContent::Center,
                  direction: match icon_pos {
                    Position::Left | Position::Right => Direction::Horizontal,
                    Position::Top | Position::Bottom => Direction::Vertical,
                  },
                  reverse: matches!(icon_pos, Position::Right | Position::Bottom),
                  @ { icon_widget }
                  @ { label_widget }
                }
              }
            }
          }
        }
      })
  }
}

impl ComposeChild for Tabs {
  type Child = Vec<Tab>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    let mut headers = vec![];
    let mut panes = vec![];

    for tab in child.into_iter() {
      let Tab { label: header, child: pane } = tab;
      headers.push((header.icon, header.text));
      panes.push(pane.child())
    }

    fn_widget! {
      let tabs_style = TabsStyle::of(ctx!());
        let TabsStyle {
          extent_only_icon,
          extent_only_label,
          extent_with_both,
          active_color,
          indicator,
          ..
        } = tabs_style.clone();
        let has_icon = headers.iter().any(|item| item.0.is_some());
        let has_label = headers.iter().any(|item| item.1.is_some());
        let extent = match (has_icon, has_label) {
          (true, true) => extent_with_both,
          (false, true) => extent_only_label,
          (true, false) => extent_only_icon,
          (false, false) => 0.
        };
        let mut flex = @Flex {
          direction: pipe!(match $this.pos {
            Position::Top | Position::Bottom => Direction::Horizontal,
            Position::Left | Position::Right => Direction::Vertical,
          })
        };
        let divider = @Divider {
          direction: pipe!(match $this.pos {
            Position::Top | Position::Bottom => Direction::Horizontal,
            Position::Left | Position::Right => Direction::Vertical,
          }),
          anchor: pipe!(
            let x = match $this.pos {
              Position::Left => $flex.layout_size().width - 1.,
              Position::Top | Position::Right | Position::Bottom => 0.,
            };
            let y = match $this.pos {
              Position::Top => $flex.layout_size().height - 1.,
              Position::Bottom | Position::Right | Position::Left => 0.,
            };
            Anchor::left_top(x, y)
          )
        };

        let indicator_decorator = @IndicatorDecorator {
          pos: pipe!($this.pos),
          extent: indicator.extent,
          rect: Rect::zero()
        };
        let header = @Stack {
          @ConstrainedBox {
            clamp: pipe!(match $this.pos {
              Position::Top | Position::Bottom => BoxClamp::fixed_height(extent),
              Position::Left | Position::Right => BoxClamp::fixed_width(extent),
            }),
            @ $flex {
              @{
                  Tabs::tab_header(
                    headers, tabs_style,
                    this.clone_writer(),
                    indicator_decorator.clone_writer()
                  )
              }
            }
          }
          @ { divider }
          @ $indicator_decorator {
            @ Container {
              background: active_color,
              size: pipe!(match $this.pos {
                Position::Top | Position::Bottom => indicator.measure.map_or(
                  Size::new($indicator_decorator.rect.width(), indicator.extent),
                  |measure| Size::new(measure, indicator.extent)
                ),
                Position::Left | Position::Right => indicator.measure.map_or(
                  Size::new(indicator.extent, $indicator_decorator.rect.height()),
                  |measure| Size::new(indicator.extent, measure)
                ),
              })
            }
          }
        };

      @TabsDecorator {
        @Flex {
          direction: pipe!(match  $this.pos {
            Position::Left | Position::Right => Direction::Horizontal,
            Position::Top | Position::Bottom => Direction::Vertical,
          }),
          reverse: pipe!{
            let pos = $this.pos;
            matches!(pos, Position::Right | Position::Bottom)
          },
          @ { header }
          @Expanded {
            @ { pipe!($this.cur_idx).map(move |idx| panes[idx].gen_widget(ctx!())) }
          }
        }
      }
    }
  }
}
