use crate::prelude::data_widget::compose_child_as_data_widget;
use crate::{impl_query_self_only, prelude::*};

#[derive(Declare)]
pub struct Tab {}

impl ComposeChild for Tab {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized,
  {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for Tab {
  impl_query_self_only!();
}

#[derive(Declare)]
pub struct Pane {}

impl ComposeChild for Pane {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized,
  {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for Pane {
  impl_query_self_only!();
}

#[derive(Default, Declare)]
pub struct Tabs {
  #[declare(default = 0)]
  pub cur_idx: usize,
}

impl ComposeChild for Tabs {
  type Child = Vec<Widget>;
  fn compose_child(this: StateWidget<Self>, children: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let mid = children.len();
    let mut tabs = vec![];
    let mut panes = vec![];

    for (i, w) in children.into_iter().enumerate() {
      if i < mid {
        tabs.push(w);
      } else {
        panes.push(w);
      }
    }

    widget! {
      track {
        this: this.into_stateful()
      }

      Column {
        Row {
          ExprWidget {
            expr: tabs.into_iter()
              .map(|tab| {
                widget! {
                  Expanded {
                    flex: 1.,
                    ExprWidget {
                      expr: tab
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
