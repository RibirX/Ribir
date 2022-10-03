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

#[derive(PartialEq)]
enum TabMode {
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
        mount_task_cnt: Stateful::new(0),
        tab_mode: Stateful::new(TabMode::All),
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
              text: String::from("Add Task"),
            }
          }
          SizedBox {
            size: Size::new(60., 30.),
            margin: EdgeInsets::only_left(20.),
            radius: Radius::all(4.),
            border: Border::all(BorderSide { width: 1., color: Color::BLACK }),
            tap: move |_| {
              if input.text.len() > 0 {
                this.tasks.push(Task {
                  label: input.text.to_string(),
                  finished: false,
                });
                input.text = String::from("Add Task");
              }
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
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                text: "Add",
                style: TypographyTheme::of(ctx).button.text.clone(),
              }
            }
          }
        }
        
        SizedBox {
          size: Size::new(512., 400.),
          VScrollBar {
            Column {
              align_items: Align::Start,
              // when performed layout, means all task are mounted, we reset the mount count.
              performed_layout: move |_| *mount_task_cnt = 0,
              ExprWidget {
                expr: this.tasks.iter()
                .filter(|task| {
                  match *tab_mode {
                    TabMode::All => true,
                    TabMode::Active => !task.finished,
                    TabMode::Completed => task.finished,
                  }
                })
                .enumerate().map(|(idx, task)| {
                  let checked = task.finished;
                  let label = task.label.clone();
                  widget! {
                    track {
                      mount_idx: Stateful::new(0),
                      visible_delete: Stateful::new(false),
                    }
                    SizedBox {
                      size: Size::new(400., 48.),
                      Row {
                        id: task,
                        align_items: Align::Center,
                        margin: EdgeInsets::vertical(4.),
                        mounted: move |_| {
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
                          // visible: *visible_delete,
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
                    }
                    on checkbox.checked  ~> this2.silent().tasks[idx].finished
                    Animate  {
                      id: mount_animate,
                      from: State { task.transform: Transform::translation(-400., 0. )},
                      transition: Transition {
                        delay: (*mount_idx + 1) * Duration::from_millis(100),
                        duration: Duration::from_millis(150),
                        easing: easing::EASE_IN,
                      }
                    }
                    on task {
                      mounted: move |_| mount_animate.run()
                    }
                  }
                }).collect::<Vec<_>>()
              }
            }
          }
        }

        Row {
          background: Brush::Color(Color::BURLYWOOD),
          Expanded {
            flex: 1.,
            Text {
              h_align: HAlign::Center,
              tap: move |_| {
                if *tab_mode != TabMode::All {
                  *tab_mode = TabMode::All;
                }
              },
              text: "All",
              style: TextStyle {
                foreground: if *tab_mode == TabMode::All {
                  Brush::Color(Color::RED)
                } else {
                  Brush::Color(Color::BLACK)
                },
                ..Default::default()
              }
            }
          }
          Expanded {
            flex: 1.,
            Text {
              h_align: HAlign::Center,
              tap: move |_| {
                if *tab_mode != TabMode::Active {
                  *tab_mode = TabMode::Active;
                }
              },
              text: "Active",
              style: TextStyle {
                foreground: if *tab_mode == TabMode::Active {
                  Brush::Color(Color::RED)
                } else {
                  Brush::Color(Color::BLACK)
                },
                ..Default::default()
              }
            }
          }
          Expanded {
            flex: 1.,
            Text {
              h_align: HAlign::Center,
              tap: move |_| {
                if *tab_mode != TabMode::Completed {
                  *tab_mode = TabMode::Completed;
                }
              },
              text: "Completed",
              style: TextStyle {
                foreground: if *tab_mode == TabMode::Completed {
                  Brush::Color(Color::RED)
                } else {
                  Brush::Color(Color::BLACK)
                },
                ..Default::default()
              }
            }
          }
        }
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
    ],
  }
  .into_stateful();

  Application::run(todo.into_widget());
}
