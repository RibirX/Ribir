use ribir::prelude::{*, svgs};

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
      // split this to avoid mutable borrow conflict in `DynWidget`.
      states {
        this: this.into_stateful(),
      }
      Column {
        margin: EdgeInsets::all(10.),
        Row {
          margin: EdgeInsets::only_bottom(10.),

          Container {
            size: Size::new(240., 30.),
            border: Border::only_bottom(BorderSide { width:1., color: Palette::of(ctx).surface_variant() }),
            Input {
              id: input,
              placeholder: String::from("Todo"),
            }
          }
          Button {
            margin: EdgeInsets::only_left(20.),
            tap: move |_| {
              this.tasks.push(Task {
                label: input.text_in_show(),
                finished: false,
              });
              input.text = String::default();
            },
            Leading { Icon { svgs::ADD } }
            ButtonText::new("ADD")
          }
        }

        Tabs {
          id: tabs,
          margin: EdgeInsets::only_top(20.),
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("All"),
                is_active: tabs.cur_idx == 0,
              }
            }
            TabPane {
              DynWidget {
                dyns: TodoMVP::pane(this.clone_stateful(), |_| true)
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
              DynWidget {
                dyns: TodoMVP::pane(this.clone_stateful(), |task| !task.finished)
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
              DynWidget {
                dyns: TodoMVP::pane(this.clone_stateful(), |task| task.finished)
              }
            }
          }
        }
      }
    }
  }
}

impl TodoMVP {
  fn pane(this: Stateful<Self>, cond: impl Fn(&Task) -> bool + 'static) -> Widget {
    widget! {
      states { this, this2: this.clone() }
      VScrollBar {
        Lists {
          padding: EdgeInsets::vertical(8.),
          DynWidget {
            dyns: this.tasks.iter()
              .enumerate()
              .filter(|(_, task)| { cond(task) })
              .map(|(idx, task)| {
              let checked = task.finished;
              let label = task.label.clone();
              widget! {
                ListItem {
                  id: item,
                  HeadlineText::new(label)
                  Leading {
                    Checkbox {
                      id: checkbox,
                      checked,
                      margin: EdgeInsets::vertical(4.),
                    }
                  }
                  Trailing {
                    Icon {
                      visible: item.mouse_hover(),
                      tap: move |_| {
                          this2.tasks.remove(idx);
                      },
                      svgs::CLOSE
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

#[derive(Debug, Declare)]
struct TabText {
  is_active: bool,
  tab_text: String,
}

impl Compose for TabText {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states {
        this: this.into_stateful()
      }
      env {
        let primary = Palette::of(ctx).primary();
        let on_surface_variant = Palette::of(ctx).on_surface_variant();
        let text_style = TypographyTheme::of(ctx).body1.text.clone();
      }
      Text {
        text: this.tab_text.clone(),
        padding: EdgeInsets::vertical(6.),
        h_align: HAlign::Center,
        style: TextStyle {
          foreground: if this.is_active { Brush::Color(primary) } else { Brush::Color(on_surface_variant) },
          ..text_style.clone()
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
