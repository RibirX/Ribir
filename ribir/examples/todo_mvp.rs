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

impl Compose for TodoMVP {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      // split this to avoid mutable borrow conflict in `ExprWidget`.
      track {
        this: this.into_stateful(),
        this2: this.clone(),
        mount_task_cnt: Stateful::new(0),
       }
      VScrollBar {
        Column {
          align_items: Align::Start,
          // when performed layout, means all task are mounted, we reset the mount count.
          performed_layout: move |_| *mount_task_cnt = 0,
          ExprWidget {
            expr: this.tasks.iter().enumerate().map(|(idx, task)| {
              let checked = task.finished;
              let label = task.label.clone();
              widget! {
                track { mount_idx: Stateful::new(0) }
                Row {
                  id: task,
                  margin: EdgeInsets::vertical(4.),
                  mounted: move |_| {
                    *mount_idx = *mount_task_cnt;
                    *mount_task_cnt +=1;
                  },
                  Checkbox { id: checkbox, checked }
                  Text {
                    text: label,
                    margin: EdgeInsets::vertical(4.)
                  }
                }
                change_on checkbox.checked ~> this2.silent().tasks[idx].finished
                Animate {
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
