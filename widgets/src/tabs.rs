use crate::prelude::*;
use ribir_core::prelude::*;

/// Tabs usage
///
/// # Example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let tabs = widget! {
///   Tabs {
///     Tab {
///       TabItem {
///         svgs::HOME
///         Label::new("Home")
///       }
///       TabPane {
///         Text { text: "content" }
///       }
///     }
///     Tab {
///       TabItem {
///         svgs::HOME
///         Label::new("Home")
///       }
///       TabPane {
///         Text { text: "content" }
///       }
///     }
///   }
/// };
///
/// // bottom tabs
/// let bottom_tabs = widget! {
///   Tabs {
///     pos: Position::Bottom,
///     Tab {
///       TabItem {
///         svgs::HOME
///         Label::new("Home")
///       }
///       TabPane {
///         Text { text: "content" }
///       }
///     }
///     Tab {
///       TabItem {
///         svgs::HOME
///         Label::new("Home")
///       }
///       TabPane {
///         Text { text: "content" }
///       }
///     }
///   }
/// };
/// ```
#[derive(Declare, Clone)]
pub struct Tabs {
  #[declare(default = Position::Top)]
  pub pos: Position,
  #[declare(default = 0)]
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

impl CustomTheme for TabsStyle {}

#[derive(Declare)]
pub struct TabsDecorator {}

impl ComposeStyle for TabsDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

#[derive(Template)]
pub struct Tab {
  header: TabItem,
  pane: WidgetOf<TabPane>,
}

#[derive(Template)]
pub struct TabItem {
  icon: Option<NamedSvg>,
  label: Option<State<Label>>,
}

#[derive(Declare, SingleChild)]
pub struct TabPane;

#[derive(Declare)]
pub struct TabDecorator {}

impl ComposeStyle for TabDecorator {
  type Host = Widget;

  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

#[derive(Declare)]
pub struct IndicatorDecorator {
  pub pos: Position,
  pub rect: Rect,
  pub extent: f32,
}

impl ComposeStyle for IndicatorDecorator {
  type Host = Widget;

  #[inline]
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

impl Tabs {
  fn tab_header(
    headers: Vec<(Option<NamedSvg>, Option<State<Label>>)>,
    tabs_style: TabsStyle,
    tabs: Stateful<Tabs>,
    indicator: Stateful<IndicatorDecorator>,
  ) -> impl Iterator<Item = Widget> {
    let TabsStyle {
      icon_size: size,
      icon_pos,
      active_color,
      foreground,
      label_style,
      ..
    } = tabs_style;
    headers
      .into_iter()
      .enumerate()
      .map(move |(idx, (icon, label))| {
        let icon_widget = icon.map(|icon| {
          widget! {
            Icon { size, DynWidget::from(icon) }
          }
        });

        let active_color = active_color.clone();
        let foreground = foreground.clone();
        let label_style = label_style.clone();
        let label_widget = label.map(|label| {
          widget! {
            states {
              tabs: tabs.clone(),
              text: label.into_readonly(),
            }
            Text {
              text: text.0.clone(),
              foreground: match tabs.cur_idx == idx {
                true => active_color.clone(),
                false => foreground.clone(),
              },
              style: label_style,
            }
          }
        });
        let indicator = indicator.clone();
        widget! {
          states { tabs: tabs.clone() }
          Expanded {
            id: tab_header,
            flex: 1.,
            on_tap: move |_| if tabs.cur_idx != idx {
              tabs.cur_idx = idx;
            },
            Flex {
              align_items: Align::Center,
              justify_content: JustifyContent::Center,
              direction: match icon_pos {
                Position::Left | Position::Right => Direction::Horizontal,
                Position::Top | Position::Bottom => Direction::Vertical,
              },
              reverse: matches!(icon_pos, Position::Right | Position::Bottom),
              DynWidget::from(icon_widget)
              // todo: insert `Spacer`
              DynWidget::from(label_widget)
            }
          }
          finally {
            let_watch!((tabs.cur_idx == idx, tab_header.layout_rect()))
              .filter_map(|(active, rect)| active.then_some(rect))
              .subscribe(move |v| indicator.silent_ref().rect = v);
          }
        }
      })
  }
}

impl ComposeChild for Tabs {
  type Child = Vec<Tab>;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let mut headers = vec![];
    let mut panes = vec![];

    for tab in child.into_iter() {
      let Tab { header, pane } = tab;
      headers.push((header.icon, header.label));
      panes.push(pane.child);
    }

    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let tabs_style = TabsStyle::of(ctx);
        let TabsStyle {
          extent_only_icon,
          extent_only_label,
          extent_with_both,
          active_color,
          indicator,
          ..
        } = tabs_style.clone();
        let tabs_style = tabs_style.clone();
        let has_icon = headers.iter().any(|item| item.0.is_some());
        let has_label = headers.iter().any(|item| item.1.is_some());
        let extent = match (has_icon, has_label) {
          (true, true) => extent_with_both,
          (false, true) => extent_only_label,
          (true, false) => extent_only_icon,
          (false, false) => 0.
        };
        let mut panes = panes.into_iter()
          .enumerate()
          .map(move |(idx, pane)| {
            widget! {
              Expanded {
                flex: 1.,
                DynWidget {
                  visible: this.cur_idx == idx,
                  dyns: pane,
                }
              }
            }
          });
        let mut header = widget! {
          Stack {
            ConstrainedBox {
              clamp: match this.pos {
                Position::Top | Position::Bottom => BoxClamp::fixed_height(extent),
                Position::Left | Position::Right => BoxClamp::fixed_width(extent),
              },
              Flex {
                id: flex,
                direction: match this.pos {
                  Position::Top | Position::Bottom => Direction::Horizontal,
                  Position::Left | Position::Right => Direction::Vertical,
                },
                Tabs::tab_header(
                  headers, tabs_style.clone(),
                  no_watch!(this.clone_stateful()),
                  no_watch!(indicator_decorator.clone_stateful()),
                )
              }
            }
            Divider {
              direction: match this.pos {
                Position::Top | Position::Bottom => Direction::Horizontal,
                Position::Left | Position::Right => Direction::Vertical,
              },
              left_anchor: match this.pos {
                Position::Left => flex.layout_size().width - 1.,
                Position::Top | Position::Right | Position::Bottom => 0.,
              },
              top_anchor: match this.pos {
                Position::Top => flex.layout_size().height - 1.,
                Position::Bottom | Position::Right | Position::Left => 0.,
              },
            }
            IndicatorDecorator {
              id: indicator_decorator,
              pos: this.pos,
              extent: indicator.extent,
              rect: Rect::zero(),
              Container {
                background: active_color.clone(),
                size: match this.pos {
                  Position::Top | Position::Bottom => indicator.measure.map_or(
                    Size::new(indicator_decorator.rect.width(), indicator.extent),
                    |measure| Size::new(measure, indicator.extent)
                  ),
                  Position::Left | Position::Right => indicator.measure.map_or(
                    Size::new(indicator.extent, indicator_decorator.rect.height()),
                    |measure| Size::new(indicator.extent, measure)
                  ),
                }
              }
            }
          }
        };
      }
      TabsDecorator {
        Flex {
          direction: match this.pos {
            Position::Left | Position::Right => Direction::Horizontal,
            Position::Top | Position::Bottom => Direction::Vertical,
          },
          reverse: matches!(this.silent_ref().pos, Position::Right | Position::Bottom),
          DynWidget::from(header)
          DynWidget::from(panes)
        }
      }
    }
  }
}
