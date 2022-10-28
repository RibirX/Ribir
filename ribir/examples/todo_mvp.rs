use ribir::prelude::*;
use std::time::Duration;
use self::text::ArcStr;

#[derive(Debug, Clone, PartialEq)]
struct Task {
  finished: bool,
  label: String,
}
#[derive(Debug)]
struct TodoMVP {
  tasks: Vec<Task>,
}

impl Compose for TodoMVP {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      // split this to avoid mutable borrow conflict in `ExprWidget`.
      track {
        this: this.into_stateful(),
      }
      Column {
        margin: EdgeInsets::all(10.),
        Row {
          margin: EdgeInsets::only_bottom(10.),
          SizedBox {
            size: Size::new(240., 30.),
            border: Border::only_bottom(BorderSide { width:1., color: Color::BLACK }),
            Input {
              id: input,
              placeholder: String::from("Add Task"),
            }
          }
          SizedBox {
            size: Size::new(60., 30.),
            margin: EdgeInsets::only_left(20.),
            radius: Radius::all(4.),
            border: Border::all(BorderSide { width: 1., color: Color::BLACK }),
            tap: move |_| {
              this.tasks.push(Task {
                label: input.text_in_show(),
                finished: false,
              });
              input.text = String::default();
            },
            Row {
              Icon {
                ExprWidget {
                  expr: {
                    icons::ADD.of_or_miss(ctx.theme())
                  }
                }
              }
              Text {
                text: "Add",
                style: TypographyTheme::of(ctx).button.text.clone(),
              }
            }
          }
        }

        Tabs {
          id: tabs,
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("All"),
                is_active: tabs.cur_idx == 0,
              }
            }
            TabPane {
              ExprWidget {
                expr: TodoMVP::pane(this.clone_stateful(), |task| true)
              }
            }
          }
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("Active"),
                is_active: tabs.cur_idx == 1,
              }
            }
            TabPane {
              ExprWidget {
                expr: TodoMVP::pane(this.clone_stateful(), |task| !task.finished)
              }
            }
          }
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("Completed"),
                is_active: tabs.cur_idx == 2,
              }
            }
            TabPane {
              ExprWidget {
                expr: TodoMVP::pane(this.clone_stateful(), |task| task.finished)
              }
            }

          }
        }

        
      }
    }
  }
}

impl TodoMVP {
  fn pane(this: Stateful<Self>, cond: impl Fn(&Task)-> bool + 'static) -> Widget{
    widget!{
      track { this, this2: this.clone() }
      VScrollBar {
        background: Brush::Color(Color::BURLYWOOD),

        Column {
          align_items: Align::Start,
          padding: EdgeInsets::all(8.),
          ExprWidget {
            expr: this.tasks.iter()
              .filter(|task| { cond(task) })
              .enumerate().map(|(idx, task)| {
              let checked = task.finished;
              let label = task.label.clone();
              widget! {
                track {
                  visible_delete: Stateful::new(false),
                }
                Row {
                  align_items: Align::Center,
                  margin: EdgeInsets::vertical(4.),

                  mounted: move |_, _| {
                    *mount_idx = *mount_task_cnt;
                    *mount_task_cnt +=1;
                  },
                  pointer_enter: move |_| { *visible_delete = true; },
                  pointer_leave: move |_| { *visible_delete = false; },
                  Checkbox { id: checkbox, checked }
                  Expanded {
                    flex: 1.,
                    Text {
                      text: label,
                      margin: EdgeInsets::vertical(4.)
                    }
                  }
                  Icon {
                    visible: *visible_delete,
                    tap: move |_| {
                      this2.tasks.remove(idx);
                    },
                    ExprWidget {
                      expr: {
                        icons::CLOSE.of_or_miss(ctx.theme())
                      }
                    }
                  }
                }
                on task {
                  mounted: move |_, _| mount_animate.run()
                }
                on checkbox.checked { change: move |(_, after)| this2.silent().tasks[idx].finished = after }
              }
            }).collect::<Vec<_>>()
          }
        }
      }
    }
  }
}

#[derive(Debug, Declare)]
struct TabText {
  is_active: bool,
  tab_text: String,
}

impl Compose for TabText {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      track {
        this: this.into_stateful()
      }
      Text {
        text: ArcStr::from(String::from(this.tab_text.as_str()).as_str()),
        padding: EdgeInsets::all(4.),
        h_align: HAlign::Center,
        style: TextStyle {
          foreground: if this.is_active { Brush::Color(Color::RED) } else { Brush::Color(Color::BLACK) },
          ..Default::default()
        },
      }
    }
  }
}

fn main() {
  env_logger::init();

  let todo = TodoMVP {
    tasks: vec![
      Task {
        finished: true,
        label: "Implement Checkbox".to_string(),
      },
      Task {
        finished: true,
        label: "Support Scroll".to_string(),
      },
      Task {
        finished: false,
        label: "Support Virtual Scroll".to_string(),
      },
      Task {
        finished: false,
        label: "Support data bind".to_string(),
      },
    ],
  }
  .into_stateful();

  app::run(todo.into_widget());
}
