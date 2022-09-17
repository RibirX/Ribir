use std::time::Duration;

use ribir::prelude::*;

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
          on_performed_layout: move |_| *mount_task_cnt = 0,
          ExprWidget {
            expr: this.tasks.iter().enumerate().map(|(idx, task)| {
              let checked = task.finished;
              let label = task.label.clone();
              widget! {
                track { mount_idx: Stateful::new(0) }
                Row {
                  id: task,
                  margin: EdgeInsets::vertical(4.),
                  on_mounted: move |_| {
                    *mount_idx = *mount_task_cnt;
                    *mount_task_cnt +=1;
                  },
                  Checkbox { id: checkbox, checked }
                  Text {
                    text: label,
                    margin: EdgeInsets::vertical(4.)
                  }
                }
                dataflows {
                  checkbox.checked ~> this2.silent().tasks[idx].finished
                }
                animations {
                  task.on_mounted: Animate  {
                    from: State { task.transform: Transform::translation(-500., 0. )},
                    transition: Transition {
                      delay: (*mount_idx + 1) * Duration::from_millis(50) ,
                      duration: Duration::from_millis(200),
                      easing: easing::EASE_IN,
                    }
                  }
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

  Application::new().run(todo.into_widget());
}
