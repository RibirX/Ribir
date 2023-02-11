use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Default, Declare)]
pub struct Tabs {
  #[declare(default = 0)]
  pub cur_idx: usize,
}

#[derive(Declare, Debug)]
pub struct InkBarStyle {
  pub ink_bar_rect: Rect,
}

impl ComposeStyle for InkBarStyle {
  type Host = Option<Widget>;
  #[inline]
  fn compose_style(_: Stateful<Self>, host: Option<Widget>) -> Widget {
    assert!(host.is_none());
    Void.into_widget()
  }
}

#[derive(Clone, Declare)]
pub struct TabStyle {
  #[declare(default=Palette::of(ctx).primary())]
  pub color: Color,
}

impl ComposeStyle for TabStyle {
  type Host = Widget;
  #[inline]
  fn compose_style(_: Stateful<Self>, style: Self::Host) -> Widget { style }
}

#[derive(Template)]
pub struct Tab {
  header: TabHeader,
  pane: WidgetOf<TabPane>,
}

#[derive(Declare, SingleChild)]
pub struct TabPane;

#[derive(Template)]
pub struct TabHeader {
  header_text: Label,
  icon: Option<WidgetOf<Leading>>,
}

impl ComposeChild for Tabs {
  type Child = Vec<Tab>;

  fn compose_child(this: State<Self>, children: Self::Child) -> Widget {
    let mut headers = vec![];
    let mut panes = vec![];

    for tab in children.into_iter() {
      let Tab { header, pane } = tab;
      headers.push((header.icon, header.header_text));
      panes.push(pane.child);
    }

    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let border_color = Palette::of(ctx).surface_variant();
        let primary = Palette::of(ctx).primary();
        let on_surface_variant = Palette::of(ctx).on_surface_variant();
        let normal_text_style = TextStyle {
          foreground: Brush::Color(on_surface_variant),
          ..TypographyTheme::of(ctx).body1.text.clone()
        };
        let active_text_style = TextStyle {
          foreground: Brush::Color(primary),
          ..TypographyTheme::of(ctx).body1.text.clone()
        };
      }
      Column {
        Stack {
          Row {
            border: Border::only_bottom(BorderSide {
              width: 1., color: border_color
            }),
            DynWidget {
              dyns: headers.into_iter()
                .enumerate()
                .map(move |(idx, (_, text))| {
                  let normal_text_style = normal_text_style.clone();
                  let active_text_style = active_text_style.clone();
                  widget! {
                    Expanded {
                      id: tab_header,
                      flex: 1.,
                      on_tap: move |_| {
                        if this.cur_idx != idx {
                          this.cur_idx = idx;
                        }
                      },
                      TabStyle {
                        DynWidget {
                          padding: EdgeInsets::vertical(6.),
                          h_align: HAlign::Center,
                          dyns: {
                            let style =  if this.cur_idx == idx {
                              active_text_style.clone()
                            } else {
                              normal_text_style.clone()
                            };
                            Text { text: text.0.clone(), style }
                          }
                        }
                      }
                    }
                    finally {
                      let_watch!((this.cur_idx == idx, tab_header.layout_rect()))
                        .filter_map(|(active, rect): (bool, Rect)| active.then_some(rect))
                        .subscribe(move |v| { ink_bar.ink_bar_rect = v });
                    }
                  }
                })
            }
          }
          InkBarStyle {
            id: ink_bar,
            ink_bar_rect: Rect::zero()
          }
        }

        DynWidget {
          dyns: panes.into_iter()
            .enumerate()
            .map(move |(idx, pane)| {
              widget! {
                DynWidget {
                  visible: this.cur_idx == idx,
                  dyns: pane
                }
              }
            })
        }

      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn compose_tabs() {
    widget! {
      Tabs {
        Tab {
          TabHeader {
            Label::new("test")
          }
          TabPane { Void {} }
        }
      }
    };
  }
}
