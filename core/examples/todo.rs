#![feature(negative_impls)]
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

impl StatefulCombination for Todos {
  #[widget]
  fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    let this_ref = unsafe { this.state_ref() };
    widget! {
      declare Column {
        h_align: CrossAxisAlign::Start,
        this.tasks.iter().enumerate().map(|(idx, task)|{
          widget!{
            declare Row {
              margin: EdgeInsets::vertical(4.),
              Checkbox{
                id: checkbox,
                checked: task.finished
              }
              Text {
                text:task.label.clone(),
                margin: EdgeInsets::vertical(4.)
              }
            }
            dataflows {
              checkbox.checked ~> this_ref.silent().tasks[idx].finished;
            }
          }
        })
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

  Application::new().run(todo.box_it(), None);
}
