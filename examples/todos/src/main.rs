use holiday::{
  prelude::*,
  widget::{Column, Row},
};

#[derive(Debug, Clone)]
struct Task {
  finished: bool,
  label: String,
}
#[derive(Debug)]
struct Todos {
  tasks: Vec<Task>,
}

impl_widget_for_combination_widget!(Todos);

impl CombinationWidget for Todos {
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    self
      .tasks
      .iter()
      .map(|task| task.clone().with_margin(EdgeInsets::vertical(4.)).box_it())
      .collect::<Column>()
      .with_cross_align(CrossAxisAlign::Start)
      .box_it()
  }
}

impl CombinationWidget for Task {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    Row::default()
      .push(
        Checkbox::from_theme(ctx.theme())
          .with_checked(self.finished)
          .with_margin(EdgeInsets::horizontal(4.))
          .box_it(),
      )
      .push(Text(self.label.clone()).box_it())
      .box_it()
  }
}

impl_widget_for_combination_widget!(Task);
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
