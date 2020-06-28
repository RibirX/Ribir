use holiday::{prelude::*, widget::RowColumn};

fn main() {
  let todo = RowColumn::column(vec![
    RowColumn::row(
      (0..15)
        .map(|i| Text(format!("FirstRow {}", i)).into())
        .collect(),
    )
    .into(),
    RowColumn::row(
      (0..1)
        .map(|i| Text(format!("SecondRow {}", i)).into())
        .collect(),
    )
    .into(),
    RowColumn::row(
      (0..3)
        .map(|i| Text(format!("ThirdRow {}", i)).into())
        .collect(),
    )
    .into(),
  ]);
  Application::new().run(todo);
}
