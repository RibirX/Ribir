use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, Clone)]
pub struct Tabs {
  #[declare(default = 0)]
  pub cur_idx: usize,
}

#[derive(Clone)]
pub struct IndicatorConfig {
  pub measure: Option<f32>,
  pub extent: f32,
  pub radius: Option<Radius>,
}

#[derive(Clone)]
pub struct TabsStyle {
  pub extent: f32,
  pub direction: Direction,
  pub icon_size: Size,
  pub icon_pos: Position,
  pub active_color: Brush,
  pub normal_color: Brush,
  pub label_style: CowArc<TextStyle>,
  pub indicator: IndicatorConfig,
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
pub struct IndicatorStyle {
  pub rect: Rect,
}

impl ComposeStyle for IndicatorStyle {
  type Host = Widget;

  #[inline]
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget { host }
}

impl Tabs {
  fn tab_header(
    headers: &Vec<(Option<NamedSvg>, Option<State<Label>>)>,
    tabs_style: TabsStyle,
    tabs: Stateful<Tabs>,
    indicator: Stateful<IndicatorStyle>,
  ) -> Vec<Widget> {
    let TabsStyle {
      icon_size,
      icon_pos,
      active_color,
      normal_color,
      label_style,
      ..
    } = tabs_style;
    headers
      .into_iter()
      .enumerate()
      .map(move |(idx, (icon, label))| {
        let icon_widget = Option::map(icon.clone(), |icon| {
          widget! {
            Icon {
              size: icon_size,
              DynWidget::from(icon)
            }
          }
        });
        let active_color = active_color.clone();
        let normal_color = normal_color.clone();
        let label_style = label_style.clone();
        let label_widget = Option::map(label.clone(), |label| {
          let tabs = tabs.clone();
          widget! {
            states {
              tabs,
              text: label.into_readonly(),
            }
            Text {
              text: text.0.clone(),
              foreground: match tabs.cur_idx == idx {
                true => active_color.clone(),
                false => normal_color.clone(),
              },
              style: label_style.clone(),
            }
          }
        });
        let indicator = indicator.clone();
        let tabs = tabs.clone();
        widget! {
          states { tabs }
          Expanded {
            id: tab_header,
            flex: 1.,
            on_tap: move |_| {
              if tabs.cur_idx != idx {
                tabs.cur_idx = idx;
              }
            },
            DynWidget {
              dyns: match icon_pos {
                Position::Top | Position::Bottom => {
                  widget! {
                    Column {
                      align_items: Align::Center,
                      justify_content: JustifyContent::Center,
                      DynWidget {
                        dyns: match icon_pos {
                          Position::Top => [ icon_widget, label_widget ],
                          Position::Bottom => [ label_widget, icon_widget ],
                          _ => unreachable!(""),
                        }
                      }
                    }
                  }
                }
                Position::Left | Position::Right => {
                  widget! {
                    Row {
                      align_items: Align::Center,
                      justify_content: JustifyContent::Center,
                      DynWidget {
                        dyns: match icon_pos {
                          Position::Left => [ icon_widget, label_widget ],
                          Position::Right => [ label_widget, icon_widget ],
                          _ => unreachable!(""),
                        }
                      }
                    }
                  }
                }
              }
            }
          }
          finally {
            let_watch!((tabs.cur_idx == idx, tab_header.layout_rect()))
              .filter_map(|(active, rect): (bool, Rect)| active.then_some(rect))
              .subscribe(move |v| {
                indicator.silent_ref().rect = v
              });
          }
        }
      })
      .collect::<Vec<_>>()
  }
}

impl ComposeChild for Tabs {
  type Child = Vec<Tab>;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let this = this.into_writable();
    let mut headers = vec![];
    let mut panes = vec![];

    for tab in child.into_iter() {
      let Tab { header, pane } = tab;
      headers.push((header.icon, header.label));
      panes.push(pane.child);
    }

    widget! {
      states { this }
      init ctx => {
        let tabs_style = TabsStyle::of(ctx);
        let TabsStyle {
          extent,
          direction,
          active_color,
          indicator,
          ..
        } = tabs_style.clone();
        let tabs_style = tabs_style.clone();
      }
      TabsDecorator {
        DynWidget {
          dyns: {
            match direction {
              Direction::Horizontal => {
                widget! {
                  Column {
                    DynWidget::from(panes.into_iter()
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
                      }))
                      Stack {
                        ConstrainedBox {
                          clamp: BoxClamp::fixed_height(extent),
                          Row {
                            id: row,
                            Tabs::tab_header(
                              &headers, tabs_style.clone(), this.clone_stateful(), indicator_style.clone_stateful(),
                            )
                          }
                        }
                        Divider {
                          top_anchor: row.layout_size().height - 1.,
                          direction: Direction::Horizontal,
                        }
                        IndicatorStyle {
                          id: indicator_style,
                          rect: Rect::zero(),
                          BoxDecoration {
                            top_anchor: row.layout_size().height - indicator.extent,
                            background: active_color.clone(),
                            border_radius: indicator.radius,
                            Container {
                              size: indicator.measure.map_or(
                                Size::new(indicator_style.rect.width(), indicator.extent),
                                |measure| Size::new(measure, indicator.extent)
                              ),
                            }
                          }
                        }
                      }
                  }
                }
              }
              Direction::Vertical => {
                widget! {
                  Row {
                    ConstrainedBox {
                      clamp: BoxClamp::fixed_width(extent),
                      Stack {
                        Column {
                          id: column,
                          Tabs::tab_header(
                            &headers, tabs_style.clone(), this.clone_stateful(), indicator_style.clone_stateful(),
                          )
                        }
                        Divider {
                          left_anchor: column.layout_size().width - 1.,
                          direction: Direction::Vertical,
                        }
                        IndicatorStyle {
                          id: indicator_style,
                          rect: Rect::zero(),
                          Container {
                            left_anchor: column.layout_size().width - indicator.extent,
                            size: indicator.measure.map_or(
                              Size::new(indicator.extent, indicator_style.rect.height()),
                              |measure| Size::new(indicator.extent, measure)
                            ),
                            background: active_color.clone(),
                          }
                        }
                      }
                    }
                    DynWidget::from(panes.into_iter()
                      .enumerate()
                      .map(move |(idx, pane)| {
                        widget! {
                          Expanded {
                            visible: this.cur_idx == idx,
                            flex: 1.,
                            DynWidget::from(pane)
                          }
                        }
                      }))
                  }
                }
              }
            }
          },
        }
      }
    }
  }
}
