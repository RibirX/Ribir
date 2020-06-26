use holiday::{prelude::*, widget::Column, widget::Row};

fn main() {
  let todo = Column(vec![
    Box::new(Row(vec![
      Box::new(Row(vec![Text("FirstRow".to_string()).into()])),
      Box::new(Column(
        (0..3)
          .map(|i| Text(format!("SecondColumn {}", i)).into())
          .collect(),
      )),
      Box::new(Column(
        (0..10)
          .map(|i| Text(format!("ThirdColumn {}", i)).into())
          .collect(),
      )),
    ])),
    Box::new(Row(
      (0..1)
        .map(|i| Text(format!("SecondRow {}", i)).into())
        .collect(),
    )),
    Box::new(Row(
      (0..3)
        .map(|i| Text(format!("ThirdRow {}", i)).into())
        .collect(),
    )),
  ]);
  Application::new().run(todo);
}
