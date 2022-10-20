use ribir::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
struct Task {
  finished: bool,
  label: String,
}
#[derive(Debug)]
struct TodoMVP {
  tasks: Vec<Task>,
}

#[derive(PartialEq, Debug)]
enum TodoMode {
  All,
  Completed,
  Active,
}

impl Compose for TodoMVP {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      // split this to avoid mutable borrow conflict in `ExprWidget`.
      track {
        this: this.into_stateful(),
        this2: this.clone(),
        todo_mode: Stateful::new(TodoMode::All),
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
                    SvgIcons::of(ctx).add.clone()
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
            TabText {
              tab_text: String::from("All"),
              is_active: *todo_mode == TodoMode::All,

              tap: move |_| {
                if *todo_mode != TodoMode::All {
                  *todo_mode = TodoMode::All;
                  tabs.cur_idx = 0;
                }
              },
            }
          }
          Tab {
            TabText {
              tab_text: String::from("Active"),
              is_active: *todo_mode == TodoMode::Active,

              tap: move |_| {
                if *todo_mode != TodoMode::Active {
                  *todo_mode = TodoMode::Active;
                  tabs.cur_idx = 0;
                }
              },
            }
          }
          Tab {
            TabText {
              tab_text: String::from("Completed"),
              is_active: *todo_mode == TodoMode::Completed,

              tap: move |_| {
                if *todo_mode != TodoMode::Completed {
                  *todo_mode = TodoMode::Completed;
                  tabs.cur_idx = 0;
                }
              },
            }
          }
    
          Pane {
            VScrollBar {
              background: Brush::Color(Color::BURLYWOOD),
      
              Column {
                align_items: Align::Start,
                padding: EdgeInsets::all(8.),
                ExprWidget {
                  expr: this.tasks.iter()
                  .filter(|_| { true })
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
                              SvgIcons::of(ctx).close.clone()
                            }
                          }
                        }
                      }
                      on checkbox.checked { change: move |(_, after)| this2.silent().tasks[idx].finished = after }
                    }
                  }).collect::<Vec<_>>()
                }
              }
            }
          }
          Pane {
            VScrollBar {
              background: Brush::Color(Color::BURLYWOOD),
      
              Column {
                align_items: Align::Start,
                padding: EdgeInsetArcStrs::all(8.),
                ExprWidget {
                  expr: this.tasks.iter()
                  .filter(|task| { !task.finished })
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
                              SvgIcons::of(ctx).close.clone()
                            }
                          }
                        }
                      }
                      on checkbox.checked { change: move |(_, after)| this2.silent().tasks[idx].finished = after }
                    }
                  }).collect::<Vec<_>>()
                }
              }
            }
          }
          Pane {
            VScrollBar {
              background: Brush::Color(Color::BURLYWOOD),
      
              Column {
                align_items: Align::Start,
                padding: EdgeInsets::all(8.),
                ExprWidget {
                  expr: this.tasks.iter()
                  .filter(|task| { task.finished })
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
                              SvgIcons::of(ctx).close.clone()
                            }
                          }
                        }
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
    let tab_bottom_border = Border::only_bottom(BorderSide { width:1., color: Color::BURLYWOOD });
    let tab_bottom_default_border = Border::only_bottom(BorderSide { width:1., color: Color::GRAY });
    widget! {
      track {
        this: this.into_stateful()
      }
      Text {
        h_align: HAlign::Center,
        padding: EdgeInsets::all(4.),
        background: if this.is_active { Color::BURLYWOOD } else { Color::WHITE },
        border: if this.is_active { tab_bottom_border.clone() } else { tab_bottom_default_border.clone() },
        text: ArcStr::from(String::from(this.tab_text.as_str()).as_str()),
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
        label: "Implement Checkbox".to_string(),
      },
      Task {
        finished: true,
        label: "Implement Checkbox".to_string(),
      },
      Task {
        finished: true,
        label: "Implement Checkbox".to_string(),
      },
    ],
  }
  .into_stateful();

  Application::run(todo.into_widget());
}
