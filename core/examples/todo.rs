use holiday::{
  prelude::*,
  widget::{Column, Row},
};

#[derive(Debug, Clone)]
struct Task {
  finished: bool,
  label: String,
}
#[derive(Debug, Widget)]
struct Todos {
  tasks: Vec<Task>,
}

impl CombinationWidget for Todos {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    self
      .tasks
      .iter()
      .enumerate()
      .map(|(idx, task)| {
        let mut todos = self.state_ref_cell(ctx);
        let mut checkbox = Checkbox::from_theme(ctx.theme()).with_checked(task.finished);
        checkbox.checked_state().subscribe(move |v| {
          todos.borrow_mut().tasks[idx].finished = v.after;
        });
        Row::default()
          .push(
            checkbox
              .with_margin(EdgeInsets::horizontal(4.))
              .with_key(idx)
              .box_it(),
          )
          .push(Text(task.label.clone()).box_it())
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
  };

  Application::new().run(todo.box_it());
}
