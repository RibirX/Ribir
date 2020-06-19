use holiday::{prelude::*, widget::RowColumn};

fn main() {
  let todo = RowColumn::Column(vec![
    Box::new(RowColumn::Row(
      (0..15)
        .map(|i| Text(format!("FirstRow {}", i)).into())
        .collect(),
    )),
    Box::new(RowColumn::Row(
      (0..1)
        .map(|i| Text(format!("SecondRow {}", i)).into())
        .collect(),
    )),
    Box::new(RowColumn::Row(
      (0..3)
        .map(|i| Text(format!("ThirdRow {}", i)).into())
        .collect(),
    )),
  ]);
  Application::new().run(todo);
}
