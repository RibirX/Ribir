use crate::{prelude::*};

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

impl ComposeChild for Tabs {
  type Child = Vec<WidgetWithChild<Tab, (WidgetWithChild<TabHeader, Widget>, WidgetWithChild<TabPane, Widget>)>>;
  fn compose_child(this: StateWidget<Self>, children: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let mut headers = vec![];
    let mut panes = vec![];

    for w in children.into_iter() {
      headers.push(w.child.0.child);
      panes.push(w.child.1.child);
    }

    widget! {
      track {
        this: this.into_stateful()
      }

      Column {
        Row {
          ExprWidget {
            expr: headers.into_iter()
              .map(|header| {
                widget! {
                  Expanded {
                    flex: 1.,
                    ExprWidget {
                      expr: header
                    }
                  }
                }
              }),
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
