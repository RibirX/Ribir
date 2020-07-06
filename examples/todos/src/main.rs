use holiday::{prelude::*, widget::RowColumn};

fn main() {
  let todo = RowColumn::column(vec![
    RowColumn::row(
      (0..15)
        .map(|i| Text(format!("FirstRow {}", i)).box_it())
        .collect(),
    )
    .box_it(),
    RowColumn::row(
      (0..1)
        .map(|i| Text(format!("SecondRow {}", i)).box_it())
        .collect(),
    )
    .box_it(),
    RowColumn::row(
      (0..3)
        .map(|i| Text(format!("ThirdRow {}", i)).box_it())
        .collect(),
    )
    .box_it(),
  ]);
  Application::new().run(todo.box_it());
}
