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
  fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    let state = unsafe { this.state_ref() };
    declare! {
      Column {
        h_align: CrossAxisAlign::Start,
        this.tasks.iter().enumerate().map(|(idx, task)|{
          let state = state.clone();
          declare!{
            Row {
              margin: EdgeInsets::vertical(4.),
              Checkbox{
                id: checkbox,
                checked: task.finished,
                style: ctx.theme().checkbox.clone(),
              }
              Text{
                text:task.label.clone(),
                style: ctx.theme().typography_theme.body1.text.clone() ,
                margin: EdgeInsets::vertical(4.),
              }
            }
            data_flow!{ checkbox.checked ~> state.silent().tasks[idx].finished }
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

  Application::new().run(todo.box_it());
}
