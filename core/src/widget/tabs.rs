
use crate::{prelude::*, impl_query_self_only};
use crate::prelude::data_widget::compose_child_as_data_widget;

#[derive(Declare)]
pub struct Tab {}

impl ComposeSingleChild for Tab {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for Tab {
  impl_query_self_only!();
}

#[derive(Declare)]
pub struct Pane {}

impl ComposeSingleChild for Pane {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
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

impl ComposeMultiChild for Tabs {
  fn compose_multi_child(this: StateWidget<Self>, children: Vec<Widget>) -> Widget {

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