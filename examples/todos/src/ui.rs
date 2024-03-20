use crate::todos::{Task, Todos};
use ribir::prelude::{svgs, *};
use std::time::Duration;

impl Compose for Todos {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @Column {
        align_items: Align::Center,
        item_gap: 12.,
        @H1 { text: "Todo" }
        @ {
          input(None, move |text| {
            $this.write().new_task(text.to_string());
          })
        }
        @Tabs {
          @Tab {
            @TabItem { @{ Label::new("ALL") } }
            @TabPane { @{ task_lists(&this, |_| true) } }
          }
          @Tab {
            @TabItem { @{ Label::new("ACTIVE") } }
            @TabPane { @{ task_lists(&this, |t| !t.complete )} }
          }
          @Tab {
            @TabItem { @{ Label::new("DONE") } }
            @TabPane { @{ task_lists(&this, |t| t.complete )} }
          }
        }
      }
    }
  }
}

fn task_lists(this: &impl StateWriter<Value = Todos>, cond: fn(&Task) -> bool) -> GenWidget {
  let this = this.clone_writer();
  fn_widget! {
    let editing = Stateful::new(None);
    let stagger = Stagger::new(Duration::from_millis(100), transitions::EASE_IN_OUT.of(ctx!()));
    let c_stagger = stagger.clone_writer().into_inner();

    @VScrollBar {
      on_mounted: move |_| c_stagger.run(),
      @Lists {
        @ { pipe!($this;).map(move |_| {
          let _hint_capture_this = || $this.write();
          let mut widgets = vec![];

          for id in $this.all_tasks() {
            if $this.get_task(id).map_or(false, cond) {
              let task = this.split_writer(
                move |todos| todos.get_task(id).unwrap(),
                move |todos| todos.get_task_mut(id).unwrap(),
              );
              let item = pipe!(*$editing == Some($task.id()))
                .value_chain(|s| s.distinct_until_changed().box_it())
                .map(move |b|{
                  if b {
                    @Container {
                      size: Size::new(f32::INFINITY, 64.),
                      @{
                        let input = input(Some($task.label.clone()), move |text|{
                          $task.write().label = text.to_string();
                          *$editing.write() = None;
                        });
                        @$input {
                          v_align: VAlign::Center,
                          on_key_down: move |e| {
                            if e.key_code() == &PhysicalKey::Code(KeyCode::Escape) {
                              *$editing.write() = None;
                            }
                          }
                        }
                      }
                    }.build(ctx!())

                  } else {
                    let _hint = || $stagger.write();
                    let item = task_item_widget(task.clone_writer(), stagger.clone_writer());
                    @$item {
                      on_double_tap: move |_| *$editing.write() = Some(id)
                    }.build(ctx!())
                  }
                });

              widgets.push(item);
            }
          }
          widgets
        }) }
      }
    }
  }
  .into()
}

fn input(
  text: Option<String>,
  mut on_submit: impl FnMut(CowArc<str>) + 'static,
) -> impl WidgetBuilder {
  fn_widget! {
    let input = @Input { };
    if let  Some(text) = text {
      $input.write().set_text(&text);
    }
    @$input {
      auto_focus: true,
      margin: EdgeInsets::horizontal(24.),
      h_align: HAlign::Stretch,
      border: {
        let color = Palette::of(ctx!()).surface_variant().into();
        Border::only_bottom(BorderSide { width: 2., color })
      },
      on_key_down: move |e| {
        if e.key_code() == &PhysicalKey::Code(KeyCode::Enter) {
          on_submit($input.text().clone());
          $input.write().set_text("");
        }
      },
      @{ Placeholder::new("What do you want to do ?") }
    }
  }
}
fn task_item_widget<S>(task: S, stagger: Writer<Stagger<Box<dyn Transition>>>) -> impl WidgetBuilder
where
  S: StateWriter<Value = Task> + 'static,
  S::OriginWriter: StateWriter<Value = Todos>,
{
  let todos = task.origin_writer().clone_writer();
  fn_widget! {
    let id = $task.id();
    let mut item = @ListItem { };
    let mut stagger = $stagger.write();
    if !stagger.has_ever_run() {
      $item.write().opacity = 0.;
      let fly_in = stagger.push_state(
        (map_writer!($item.transform), map_writer!($item.opacity)),
        (Transform::translation(0., 64.), 0.),
        ctx!()
      );
      // items not displayed until the stagger animation is started.
      watch!($fly_in.is_running()).filter(|v| *v).first().subscribe(move |_| {
        $item.write().opacity = 1.;
      });
    }

    @$item {
      @{ HeadlineText(Label::new($task.label.clone())) }
      @Leading {
        @{
          let checkbox = @Checkbox { checked: pipe!($task.complete) };
          watch!($checkbox.checked)
            .distinct_until_changed()
            .subscribe(move |v| $task.write().complete = v);
          CustomEdgeWidget(checkbox.build(ctx!()))
        }
      }
      @Trailing {
        cursor: CursorIcon::Pointer,
        on_tap: move |_| $todos.write().remove(id),
        @{ svgs::CLOSE }
      }
    }
  }
}

pub fn todos() -> impl WidgetBuilder {
  let todos = State::value(Todos::load());

  // save changes to disk every 5 seconds .
  let save_todos = todos.clone_reader();
  todos
    .modifies()
    .debounce(Duration::from_secs(5), AppCtx::scheduler())
    .subscribe(move |_| {
      if let Err(err) = save_todos.read().save() {
        log::error!("Save tasks failed: {}", err);
      }
    });

  fn_widget! { todos }
}
