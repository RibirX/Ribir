use holiday::{prelude::*, widget::Row};

fn main() {
  let todo = Row(
    (0..10)
      .map(|i| Text(format!("Todo {}", i)).into())
      .collect(),
  );
  Application::new().run(todo);
}
