use ribir::prelude::*;

use crate::todos::{Task, Todos};

impl Compose for Todos {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    providers! {
      providers: [Provider::value_of_writer(this.clone_writer(), None)],
      @Column {
        align_items: Align::Center,
        item_gap: 12.,
        @H1 { text: "Todo" }
        @input(None, move |text| {
          $this.write().new_task(text.to_string());
        })
        @Expanded {
          @Tabs {
            h_align: HAlign::Stretch,
            @Tab {
              @ { "All" }
              @task_lists(this.clone_writer(), |_| true)
            }
            @Tab {
              @ { "ACTIVE" }
              @task_lists(this.clone_writer(), |t| !t.complete )
            }
            @Tab {
              @ { "DONE" }
              @task_lists(this, |t| t.complete )
            }
          }
        }
      }
    }
    .into_widget()
  }
}

fn task_lists(
  this: impl StateWriter<Value = Todos> + 'static, cond: fn(&Task) -> bool,
) -> GenWidget {
  fn_widget! {
    let editing = Stateful::new(None);
    let stagger = Stagger::new(
      Duration::from_millis(100),
      transitions::EASE_IN_OUT.of(BuildCtx::get())
    );
    let c_stagger = stagger.clone_writer();

    @Scrollbar {
      on_mounted: move |_| c_stagger.run(),
      @ {
        pipe!($this;).map(move |_| fn_widget!{
          let _hint_capture_this = || $this.write();
          let mut items = List::child_template();
          for id in $this.all_tasks() {
            if $this.get_task(id).map_or(false, cond) {
              let task = this.split_writer(
                // task will always exist, if the task is removed,
                // sthe widgets list will be rebuild.
                move |todos| PartMut::new(todos.get_task_mut(id).unwrap()),
              );
              let item = pipe!(*$editing == Some(id))
                .value_chain(|s| s.distinct_until_changed().box_it())
                .map(move |b| fn_widget!{
                  if b {
                    @Container {
                      size: Size::new(f32::INFINITY, 64.),
                      @{
                        let input = input(Some($task.label.clone()), move |text|{
                          $task.write().label = text.to_string();
                          *$editing.write() = None;
                        });
                        let mut input = FatObj::new(input);
                        @ $input {
                          v_align: VAlign::Center,
                          on_key_down: move |e| {
                            if e.key_code() == &PhysicalKey::Code(KeyCode::Escape) {
                              *$editing.write() = None;
                            }
                          }
                        }
                      }
                    }.into_widget()
                  } else {
                    let _hint = || $stagger.write();
                    let item = task_item_widget(task.clone_writer(), stagger.clone_writer());
                    let mut item = FatObj::new(item);
                    @ $item {
                      on_double_tap: move |_| *$editing.write() = Some(id)
                    }.into_widget()
                  }
                });
              items = items.with_child(@ListCustomItem {
                interactive: false,
                @{ item }
              });
            }
          }
          @List {
            select_mode: ListSelectMode::None,
            @ { items }
          }
        })
      }
    }
  }
  .into()
}

fn input(
  text: Option<String>, mut on_submit: impl FnMut(CowArc<str>) + 'static,
) -> Widget<'static> {
  fn_widget! {
    let input = @Input { auto_focus: true };
    if let Some(text) = text {
      $input.write().set_text(&text);
    }
    @ Stack {
      padding: EdgeInsets::horizontal(24.),
      @Text {
        h_align: HAlign::Stretch,
        visible: pipe!($input.text().is_empty()),
        text: "What do you want to do ?"
      }
      @ $input {
        h_align: HAlign::Stretch,
        on_key_down: move |e| {
          if e.key_code() == &PhysicalKey::Code(KeyCode::Enter) {
            on_submit($input.text().clone());
            $input.write().set_text("");
          }
        },
      }
    }
  }
  .into_widget()
}

fn task_item_widget<S>(task: S, stagger: Stateful<Stagger<Box<dyn Transition>>>) -> Widget<'static>
where
  S: StateWriter<Value = Task> + 'static,
{
  fn_widget! {
    let id = $task.id();
    let item = @ListItemChildren {
      @ {
        let mut checkbox = @Checkbox { checked: pipe!($task.complete) };
        let u = watch!($checkbox.checked)
          .distinct_until_changed()
          .subscribe(move |v| $task.write().complete = v);
        checkbox.on_disposed(move |_| u.unsubscribe());
        checkbox
      }
      @ListItemHeadline { @ { $task.label.clone() } }
      @Trailing {
        @Icon {
          cursor: CursorIcon::Pointer,
          on_tap: move |e| Provider::write_of::<Todos>(e).unwrap().remove(id),
          @ { svgs::CLOSE }
        }
      }
    }.build_tml().compose_sections();

    let mut item = FatObj::new(item);

    let mut stagger = $stagger.write();
    if !stagger.has_ever_run() {
      $item.write().opacity = 0.;
      let transform = item
        .get_transform_widget()
        .map_writer(|w| PartMut::new(&mut w.transform));
      let opacity = item
        .get_opacity_widget()
        .map_writer(|w| PartMut::new(&mut w.opacity));
      let fly_in = stagger.push_state(
        (transform, opacity),
        (Transform::translation(0., 64.), 0.),
      );
      // items not displayed until the stagger animation is started.
      watch!($fly_in.is_running()).filter(|v| *v).first().subscribe(move |_| {
        $item.write().opacity = 1.;
      });
    }

    item
  }
  .into_widget()
}

pub fn todos() -> Widget<'static> {
  let todos = if cfg!(not(target_arch = "wasm32")) {
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
    todos
  } else {
    State::value(Todos::default())
  };

  todos.into_widget()
}
