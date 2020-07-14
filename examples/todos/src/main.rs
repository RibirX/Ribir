use holiday::{
  prelude::*,
  widget::{Column, Row},
};

#[derive(Debug)]
struct Todos {}

impl CombinationWidget for Todos {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    let mut column = Column::default();
    column
      .push(Row::from_iter((0..15).map(|i| {
        let (stateful, mut state_modify) = Text(format!("FirstRow {} ", i)).into_stateful(ctx);
        stateful.on_pointer_down(move |_| state_modify.0 = state_modify.0.clone() + "1")
      })))
      .push(Row::from_iter(
        (0..10).map(|i| Text("SecondRow".to_string()).box_it()),
      ))
      .push(Row::from_iter(
        (0..3).map(|i| Text(format!("ThirdRow {} ", i)).box_it()),
      ));

    column.box_it()
  }
}
fn main() {
  env_logger::init();
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
