use ribir::prelude::{svgs, *};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
struct Task {
  id: usize,
  finished: bool,
  label: String,
}
#[derive(Debug)]
struct Todos {
  tasks: Vec<Task>,
  id_gen: usize,
}

impl Compose for Todos {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @Column {
        padding: EdgeInsets::all(10.),
        @H1 {
          margin: EdgeInsets::only_bottom(10.),
          text: "Todo",
        }
        @{
          let input = @Input { };
          let add_task = move |_: &mut _| if !$input.text().is_empty() {
              $this.write().new_task($input.text().to_string());
              $input.write().set_text("");
          };
          @Row {
            @Container {
              size: Size::new(240., 30.),
              border: {
                let color = Palette::of(ctx!()).surface_variant().into();
                Border::only_bottom(BorderSide { width:1., color })
              },
              @ $input { @{ Placeholder::new("What do you want to do ?") } }
            }
            @FilledButton {
              margin: EdgeInsets::only_left(20.),
              on_tap: add_task,
              @{ Label::new("ADD") }
            }
          }
        }
        @Tabs {
          pos: Position::Top,
          @Tab {
            @TabItem { @{ Label::new("ALL") } }
            @TabPane { @{ Self::pane(this.clone_writer(), |_| true) } }
          }
          @Tab {
            @TabItem { @{ Label::new("ACTIVE") } }
            @TabPane { @{ Self::pane(this.clone_writer(), |task| !task.finished)} }
          }
          @Tab {
            @TabItem { @{ Label::new("DONE") } }
            @TabPane { @{ Self::pane(this.clone_writer(), |task| task.finished) } }
          }
        }
      }
    }
  }
}

impl Todos {
  fn pane(this: impl StateWriter<Value = Self>, cond: fn(&Task) -> bool) -> impl WidgetBuilder {
    fn_widget! {
      // todo: pipe only for list items, not lists
      @VScrollBar { @ { pipe! {
        let mount_task_cnt = Stateful::new(0);

        @Lists {
          // when performed layout, means all task are mounted, we reset the mount count.
          on_performed_layout: move |_| *$mount_task_cnt.write() = 0,
          padding: EdgeInsets::vertical(8.),
          @ {
            $this
              .tasks
              .iter()
              .enumerate()
              .filter_map(move |(idx, task)| { cond(task).then_some(idx) })
              .map(move |idx| {
                let task = split_writer!($this.tasks[idx]);
                let mut key = @KeyWidget { key: $task.id, value: () };
                let mount_idx = Stateful::new(0);

                let mut mount_animate = @Animate {
                  transition: @Transition {
                    delay: pipe!(Duration::from_millis(100).mul_f32(*$mount_idx as f32)),
                    duration: Duration::from_millis(150),
                    easing: easing::EASE_IN,
                  }.into_inner(),
                  state: map_writer!($key.transform),
                  from: Transform::translation(-400., 0. ),
                };
                @ $key {
                  @ListItem {
                    on_mounted: move |_| if $key.is_enter() {
                      *$mount_idx.write() = *$mount_task_cnt;
                      *$mount_task_cnt.write() += 1;
                      mount_animate.run();
                    },
                    @{ HeadlineText(Label::new($task.label.clone())) }
                    @Leading {
                      @{
                        let checkbox = @Checkbox {
                          checked: pipe!($task.finished),
                          margin: EdgeInsets::vertical(4.),
                        };
                        watch!($checkbox.checked)
                          .distinct_until_changed()
                          .subscribe(move |v| $task.write().finished = v);
                        CustomEdgeWidget(checkbox.widget_build(ctx!()))
                      }
                    }
                    @Trailing {
                      cursor: CursorIcon::Hand,
                      visible: $key.mouse_hover(),
                      on_tap: move |_| { $this.write().tasks.remove(idx); },
                      @{ svgs::CLOSE }
                    }
                  }
                }
              }).collect::<Vec<_>>()
          }

        }
      }}}
    }
  }

  fn new_task(&mut self, label: String) {
    self.tasks.push(Task {
      id: self.id_gen,
      label,
      finished: false,
    });
    self.id_gen += 1;
  }
}

pub fn todos() -> impl WidgetBuilder {
  fn_widget! {
    Todos {
      tasks: vec![
        Task {
          id: 0,
          finished: true,
          label: "Implement Checkbox".to_string(),
        },
        Task {
          id: 1,
          finished: true,
          label: "Support Scroll".to_string(),
        },
        Task {
          id: 2,
          finished: false,
          label: "Support Virtual Scroll".to_string(),
        },
      ],
      id_gen: 3,
    }
  }
}
