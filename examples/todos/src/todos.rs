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
  fn compose(mut this: State<Self>) -> Widget {
    fn_widget! {
      @Column {
        padding: EdgeInsets::all(10.),
        @H1 {
          margin: EdgeInsets::only_bottom(10.),
          text: "Todo",
        }
        @{
          let mut input = @Input { };
          let add_task = move |_: &mut _| if !$input.text().is_empty() {
              $this.new_task($input.text().to_string());
              $input.set_text("");
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
            @TabPane { @{ Self::pane(this.clone_state(), |_| true) } }
          }
          @Tab {
            @TabItem { @{ Label::new("ACTIVE") } }
            @TabPane { @{ Self::pane(this.clone_state(), |task| !task.finished)} }
          }
          @Tab {
            @TabItem { @{ Label::new("DONE") } }
            @TabPane { @{ Self::pane(this.clone_state(), |task| task.finished) } }
          }
        }
      }
    }
    .into()
  }
}

impl Todos {
  fn pane(mut this: Stateful<Self>, cond: fn(&Task) -> bool) -> Widget {
    fn_widget! {

      @VScrollBar { @ { pipe! {
        let mut mount_task_cnt = Stateful::new(0);

        @Lists {
          // when performed layout, means all task are mounted, we reset the mount count.
          on_performed_layout: move |_| *$mount_task_cnt = 0,
          padding: EdgeInsets::vertical(8.),
          @ {
            let mut this2 = this.clone_state();
            let task_widget_iter = $this
              .tasks
              .iter()
              .enumerate()
              .filter_map(move |(idx, task)| { cond(task).then_some(idx) })
              .map(move |idx| {
                let mut task = partial_state!($this2.tasks[idx]);
                let mut key = @KeyWidget { key: $task.id, value: () };
                let mut mount_idx = Stateful::new(0);

                let mut mount_animate = @Animate {
                  transition: @Transition {
                    delay: pipe!(Duration::from_millis(100).mul_f32(*$mount_idx as f32)),
                    duration: Duration::from_millis(150),
                    easing: easing::EASE_IN,
                  }.into_inner(),
                  state: route_state!($key.transform),
                  from: Transform::translation(-400., 0. ),
                };
                @ $key {
                  @ListItem {
                    on_mounted: move |_| if $key.is_enter() {
                      *$mount_idx = *$mount_task_cnt;
                      *$mount_task_cnt += 1;
                      mount_animate.run();
                    },
                    @{ HeadlineText(Label::new($task.label.clone())) }
                    @Leading {
                      @{
                        let mut checkbox = @Checkbox {
                          checked: pipe!($task.finished),
                          margin: EdgeInsets::vertical(4.),
                        };
                        watch!($checkbox.checked).subscribe(move |v| $task.finished = v);
                        CustomEdgeWidget(checkbox.into())
                      }
                    }
                    @Trailing {
                      cursor: CursorIcon::Hand,
                      visible: $key.mouse_hover(),
                      on_tap: move |_| { $this2.tasks.remove(idx); },
                      @{ svgs::CLOSE }
                    }
                  }
                }
              }).collect::<Vec<_>>();
            Multi::new(task_widget_iter)
          }

        }
      }}}
    }
    .into()
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

pub fn todos() -> Widget {
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
  .into()
}
