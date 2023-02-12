use ribir::prelude::{svgs, *};
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

impl Compose for TodoMVP {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let surface_variant = Palette::of(ctx).surface_variant();
        let headline1_style = TypographyTheme::of(ctx).headline1.text.clone();
      }
      Column {
        margin: EdgeInsets::all(10.),
        Text {
          margin: EdgeInsets::only_bottom(10.),
          text: "Todo",
          style: headline1_style,
        }
        Row {
          margin: EdgeInsets::only_bottom(10.),

          Container {
            size: Size::new(240., 30.),
            border: Border::only_bottom(BorderSide { width:1., color: surface_variant }),
            Input {
              id: input,
              Placeholder::new("What do you want to do?")
            }
          }
          Button {
            margin: EdgeInsets::only_left(20.),
            on_tap: move |_| {
              let label = if input.text().is_empty() {
                String::from("Todo")
              } else {
                input.text().to_string()
              };
              this.tasks.push(Task {
                label,
                finished: false,
              });
              input.set_text("");
            },
            Leading { Icon { svgs::ADD } }
            ButtonText::new("ADD")
          }
        }

        Tabs {
          id: tabs,
          margin: EdgeInsets::only_top(10.),
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("ALL"),
                is_active: tabs.cur_idx == 0,
              }
            }
            TabPane { DynWidget {
              dyns: (tabs.cur_idx == 0).then(|| Self::pane(this, |_| true))
            }}
          }
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("ACTIVE"),
                is_active: tabs.cur_idx == 1,
              }
            }
            TabPane { DynWidget {
              dyns: (tabs.cur_idx == 1).then(|| Self::pane(this, |task| !task.finished))
            }}
          }
          Tab {
            TabHeader {
              TabText {
                tab_text: String::from("DONE"),
                is_active: tabs.cur_idx == 2,
              }
            }
            TabPane { DynWidget {
              dyns: (tabs.cur_idx == 2).then(|| Self::pane(this, |task| task.finished))
            }}
          }
        }
      }
    }
  }
}

impl TodoMVP {
  fn pane(this: StateRef<Self>, cond: fn(&Task) -> bool) -> Widget {
    let this = this.clone_stateful();
    widget! {
      states { this, mount_task_cnt: Stateful::new(0) }
      VScrollBar {
        Lists {
          // when performed layout, means all task are mounted, we reset the mount count.
          on_performed_layout: move |_| *mount_task_cnt = 0,
          padding: EdgeInsets::vertical(8.),
          DynWidget {
            dyns: {
              let tasks = this.tasks.clone();
              tasks
                .into_iter()
                .enumerate()
                .filter(move |(_, task)| { cond(task) })
                .map(move |(idx, task)| {
                  Self::task(this, task, idx, mount_task_cnt)
                })
            }
          }
        }
      }
    }
  }

  fn task(this: StateRef<Self>, task: Task, idx: usize, mount_task_cnt: StateRef<i32>) -> Widget {
    let this = this.clone_stateful();
    let mount_task_cnt = mount_task_cnt.clone_stateful();
    widget! {
      states { this, mount_task_cnt, mount_idx: Stateful::new(0) }
      KeyWidget {
        id: key,
        key: Key::from(idx),
        value: Some(task.label.clone()),
        ListItem {
          id: item,
          transform: Transform::default(),
          on_mounted: move |_| {
            if key.is_enter() {
              *mount_idx = *mount_task_cnt;
              *mount_task_cnt += 1;
              mount_animate.run();
            }
          },
          HeadlineText::new(task.label.clone())
          Leading {
            Checkbox {
              id: checkbox,
              checked: task.finished,
              margin: EdgeInsets::vertical(4.),
            }
          }
          Trailing {
            Icon {
              visible: item.mouse_hover(),
              on_tap: move |_| { this.tasks.remove(idx); },
              svgs::CLOSE
            }
          }
        }
      }
      Animate {
        id: mount_animate,
        transition: Transition {
          delay: Some(Duration::from_millis(100).mul_f32((*mount_idx + 1) as f32)),
          duration: Duration::from_millis(150),
          easing: easing::EASE_IN,
          repeat: None,
        },
        prop: prop!(item.transform),
        from: Transform::translation(-400., 0. ),
      }
      finally {
        let_watch!(checkbox.checked)
          .subscribe(move |v| this.silent().tasks[idx].finished = v);
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
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states {
        this: this.into_readonly()
      }
      init ctx => {
        let primary = Palette::of(ctx).primary();
        let on_surface_variant = Palette::of(ctx).on_surface_variant();
        let text_style = TypographyTheme::of(ctx).body1.text.clone();
      }
      Text {
        text: this.tab_text.clone(),
        padding: EdgeInsets::vertical(6.),
        h_align: HAlign::Center,
        style: TextStyle {
          foreground: if this.is_active {
            Brush::Color(primary)
          } else {
            Brush::Color(on_surface_variant)
          },
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
  };

  let app = Application::new(material::purple::light());
  let wnd = Window::builder(todo.into_widget())
    .with_inner_size(Size::new(400., 640.))
    .build(&app);
  app::run_with_window(app, wnd);
}
