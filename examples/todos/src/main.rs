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
      .push(
        (0..15)
          .map(|i| {
            let (stateful, mut state_modify) = Text(format!("FirstRow {} ", i)).into_stateful(ctx);
            stateful
              .with_cursor(CursorIcon::Grab)
              .on_pointer_down(move |_| state_modify.0 = state_modify.0.clone() + "1")
          })
          .collect::<Row>(),
      )
      .push(
        (0..10)
          .map(|i| {
            Text(format!("SecondRow {} ", i))
              .with_cursor(CursorIcon::Hand)
              .box_it()
          })
          .collect::<Row>(),
      )
      .push(
        (0..3)
          .map(|i| {
            Text(format!("ThirdRow{} ", i))
              .with_cursor(CursorIcon::Progress)
              .box_it()
          })
          .collect::<Row>(),
      );

    column.box_it()
  }
}
fn main() {
  env_logger::init();
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
