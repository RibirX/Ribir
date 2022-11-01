use crate::prelude::*;
use ribir_core::prelude::*;

#[derive(Declare, SingleChild)]
pub struct Tab;

#[derive(Declare, SingleChild)]
pub struct TabPane;

#[derive(Declare, SingleChild)]
pub struct TabHeader;

#[derive(Default, Declare)]
pub struct Tabs {
  #[declare(default = 0)]
  pub cur_idx: usize,
}

#[derive(Declare, Debug)]
pub struct InkBarStyle;

impl ComposeStyle for InkBarStyle {
  type Host = Widget;
  #[inline]
  fn compose_style(_: StateWidget<Self>, host: Widget) -> Widget { host }
}

impl ComposeChild for Tabs {
  type Child = ChildVec<
    WidgetWithChild<
      Tab,
      (
        WidgetWithChild<TabHeader, Widget>,
        WidgetWithChild<TabPane, Widget>,
      ),
    >,
  >;
  fn compose_child(this: StateWidget<Self>, children: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let mut headers = vec![];
    let mut panes = vec![];

    for w in children.into_inner().into_iter() {
      headers.push(w.child.0.child);
      panes.push(w.child.1.child);
    }

    let tab_size = panes.len();

    widget! {
      track {
        this: this.into_stateful()
      }

      Column {
        LayoutBox {
          id: stack,
          Stack {
            Row {
              border: Border::only_bottom(BorderSide {
                width: 1., color: ctx.theme().palette.primary()
              }),
              ExprWidget {
                expr: {
                  headers.into_iter()
                    .enumerate()
                    .map(move |(idx, header)| {
                      widget! {
                        Expanded {
                          flex: 1.,
                          tap: move |_| {
                            if this.cur_idx != idx {
                              this.cur_idx = idx;
                            }
                          },
                          ExprWidget {
                            h_align: HAlign::Center,
                            v_align: VAlign::Center,

                            expr: header
                          }
                        }
                      }
                    })
                }
              }
            }
            InkBarStyle {
              id: ink_bar,
              left_anchor: 0.,
              top_anchor: 0.,
              Container {
                id: ink_box,
                size: Size::new(0., 0.),
              }
            }
          }
        }

        ExprWidget {
          expr: panes.into_iter()
            .enumerate()
            .map(move |(idx, pane)| {
              widget! {
                ExprWidget {
                  visible: this.cur_idx == idx,
                  expr: pane
                }
              }
            })
        }

      }

      on this.cur_idx {
        change: move |(_, after)| {
          let width = stack.layout_width();
          let height = stack.layout_height();
          let pos = (after as f32) * width / (tab_size as f32);
          ink_bar.left_anchor = PositionUnit::Pixel(pos);
          ink_bar.top_anchor = PositionUnit::Pixel(height - 2.);
        }
      }

      on stack.layout_width() {
        change: move |(_, after)| {
          let width = after / (tab_size as f32);
          let height = 2.;
          ink_box.size = Size::new(width, height);

          let pos = (this.cur_idx as f32) * width / (tab_size as f32);
          ink_bar.left_anchor = PositionUnit::Pixel(pos);
        }
      }

      on stack.layout_height() {
        change: move |(_, after)| {
          ink_bar.top_anchor = PositionUnit::Pixel(after - 2.);
        }
      }

      change_on ink_bar.left_anchor Animate {
        transition: transitions::EASE_IN.get_from_or_default(ctx.theme()),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(0.);
          let to = to.abs_value(0.);
          PositionUnit::Pixel(from.lerp(&to, rate))
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
            Void {}
          }
          TabPane {
            Void {}
          }
        }
      }
    };
  }
}
