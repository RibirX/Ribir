use holiday::{
  prelude::*,
  widget::{Column, Row},
};

#[derive(Debug, Clone, PartialEq)]
struct Task {
  finished: bool,
  label: String,
}
#[stateful]
#[derive(Debug, Widget)]
struct Todos {
  #[state]
  tasks: Vec<Task>,
}

impl CombinationWidget for StatefulTodos {
  fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> {
    self
      .as_ref()
      .tasks
      .iter()
      .enumerate()
      .map(|(idx, task)| {
        let mut todos = self.ref_cell();
        let mut checkbox = Checkbox::from_theme(ctx.theme())
          .with_checked(task.finished)
          .into_stateful();
        checkbox.state_checked().subscribe(move |v| {
          todos.borrow_mut().tasks[idx].finished = v.after;
        });
        Row::default()
          .push(
            checkbox
              .with_margin(EdgeInsets::horizontal(4.))
              .with_key(idx),
          )
          .push(Text(task.label.clone()))
          .with_margin(EdgeInsets::vertical(4.))
          .box_it()
      })
      .collect::<Column>()
      .with_cross_align(CrossAxisAlign::Start)
      .box_it()
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
        finished: false,
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

  Application::new().run(todo);
}
