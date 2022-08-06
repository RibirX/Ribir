use ribir::prelude::*;

#[derive(Debug, Clone, PartialEq)]
struct Task {
  finished: bool,
  label: String,
}
#[derive(Debug)]
struct Todos {
  tasks: Vec<Task>,
}

impl Compose for Todos {
  fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
    widget! {
      // split this to avoid mutable borrow conflict in `ExprWidget`.
      track { this, this2: this.clone() }
      Column {
        align_items: Align::Start,
        ExprWidget {
          expr: this.tasks.iter().enumerate().map(|(idx, task)| {
            let checked = task.finished;
            let label = task.label.clone();
            widget! {
              Row {
                margin: EdgeInsets::vertical(4.),
                Checkbox { id: checkbox, checked }
                Text {
                  text: label,
                  margin: EdgeInsets::vertical(4.)
                }
              }
              dataflows {
                checkbox.checked ~> this2.silent().tasks[idx].finished
              }
            }
          })
        }
      }
    }
  }
}

fn main() {
  env_logger::init();

  let todo = Todos {
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

  Application::new().run(todo.into_widget(), None);
}
