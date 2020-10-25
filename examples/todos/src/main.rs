use holiday::{
  prelude::*,
  widget::{Column, Row},
};

#[derive(Debug)]
struct Todos {}

impl_widget_for_combination_widget!(Todos);

impl CombinationWidget for Todos {
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    Column::default()
      .push(
        (0..15)
          .map(|i| {
            let stateful = Text(format!("FirstRow {} ", i)).into_stateful();
            let mut state_ref = stateful.get_state_ref();
            stateful
              .with_cursor(CursorIcon::Text)
              .on_char(move |e| state_ref.borrow_mut().0.push(e.char))
              .box_it()
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
      )
      .box_it()
  }
}
fn main() {
  env_logger::init();
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
