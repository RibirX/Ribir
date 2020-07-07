use holiday::{prelude::*, widget::RowColumn};

#[derive(Debug)]
struct Todos {}

impl CombinationWidget for Todos {
  fn build(&self) -> BoxWidget {
    unimplemented!();
    //   RowColumn::column(vec![
    //     RowColumn::row(
    //       (0..15)
    //         .map(|i| {
    //           let text = Text(format!("FirstRow {}", i)).box_it();
    //           let (stateful, cell_ref) = text.into_stateful(ctx);
    //           let cell_ref = stateful.as_cell_ref();
    //           stateful.box_it()
    //         })
    //         .collect(),
    //     )
    //     .box_it(),
    //     RowColumn::row(
    //       (0..1)
    //         .map(|i| Text(format!("SecondRow {}", i)).box_it())
    //         .collect(),
    //     )
    //     .box_it(),
    //     RowColumn::row(
    //       (0..3)
    //         .map(|i| Text(format!("ThirdRow {}", i)).box_it())
    //         .collect(),
    //     )
    //     .box_it(),
    //   ])
  }
}
fn main() {
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
