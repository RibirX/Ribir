use holiday::{prelude::*, widget::column, widget::row};

fn main() {
  let todo = column(vec![
    Box::new(row(vec![
      Box::new(row(vec![Text("FirstRow".to_string()).into()])),
      Box::new(column(
        (0..3)
          .map(|i| Text(format!("SecondColumn {}", i)).into())
          .collect(),
      )),
      Box::new(column(
        (0..10)
          .map(|i| Text(format!("ThirdColumn {}", i)).into())
          .collect(),
      )),
    ])),
    Box::new(row(
      (0..1)
        .map(|i| Text(format!("SecondRow {}", i)).into())
        .collect(),
    )),
    Box::new(row(
      (0..3)
        .map(|i| Text(format!("ThirdRow {}", i)).into())
        .collect(),
    )),
  ]);
  Application::new().run(todo);
}
