#![feature(negative_impls)]
use ribir::prelude::*;

#[derive(Debug, Clone, PartialEq)]
struct Task {
  finished: bool,
  label: String,
}
#[stateful(custom)]
#[derive(Debug)]
struct Todos {
  #[state]
  tasks: Vec<Task>,
}

impl CombinationWidget for StatefulTodos {
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    let state = self.ref_cell();
    let tasks = &self.borrow().tasks;
    let tasks_iter = tasks.iter().enumerate();
    declare! {
      Column {
        cross_align: CrossAxisAlign::Start,
        ..<_>::default(),
        tasks_iter.map(|(idx, task)|{
          let state = state.clone();
          declare!{
            Row {
              margin: EdgeInsets::vertical(4.),
              ..<_>::default(),
              Checkbox{
                id: checkbox,
                checked: task.finished,
                style: ctx.theme().checkbox.clone(),
                ..<_>::default(),
              }
              Text{
                text:task.label.clone(),
                margin: EdgeInsets::vertical(4.),
              }
            }
            data_flow!{ checkbox.checked ~> state.silent_mut().tasks[idx].finished }
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
