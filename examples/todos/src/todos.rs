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
      @VScrollBar { @ {
        let stagger = Stagger::new(Duration::from_millis(100), transitions::EASE_IN.of(ctx!()));
        let c_stagger = stagger.clone_writer().into_inner();

        @Lists {
          on_mounted: move |_| stagger.run(),
          padding: EdgeInsets::vertical(8.),
          @ {
            pipe!($this;).map(move |_| {
              $this
              .tasks
              .iter()
              .enumerate()
              .filter_map(move |(idx, task)| { cond(task).then_some(idx) })
              .map(move |idx| {

                let mut item = @ListItem { };
                if !$c_stagger.has_ever_run() {
                  $item.write().opacity = 0.;

                  let fly_in = $c_stagger.write().push_state(
                    (map_writer!($item.transform), map_writer!($item.opacity)),
                    (Transform::translation(0., 200. ), 0.),
                    ctx!()
                  );

                  watch!($fly_in.is_running()).filter(|v| *v).first().subscribe(move |_| {
                    $item.write().opacity = 1.;
                  });
                }

                let task = split_writer!($this.tasks[idx]);
                @$item {
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
                    cursor: CursorIcon::Pointer,
                    visible: $item.mouse_hover(),
                    on_tap: move |_| { $this.write().tasks.remove(idx); },
                    @{ svgs::CLOSE }
                  }
                }

              }).collect::<Vec<_>>()
            })
          }

        }
      }}
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
