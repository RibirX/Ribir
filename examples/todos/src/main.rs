use canvas::Color;
use holiday::{prelude::*, widget::FontStyle};

#[derive(Debug)]
struct Todos {}

impl CombinationWidget for Todos {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    (0..2)
      .map(|i| {
        let (stateful, mut state_modify) = Text {
          text: Some(format!("Row {} ", i)),
          children: Some(
            (0..1)
              .map(|i| {
                Text {
                  text: Some(format!("SecondElem {} ", i)),
                  children: None,
                  style: Some(FontStyle::default().with_size(48.).with_color(Color::BLUE)),
                }
                .box_it()
              })
              .collect(),
          ),
          style: Some(
            FontStyle::default()
              .with_size(164.)
              .with_color(Color::YELLOW),
          ),
        }
        .into_stateful(ctx);
        stateful.with_cursor(CursorIcon::Text).on_char(move |e| {
          state_modify.text.as_mut().map(|text| text.push(e.char));
        })
      })
      .collect::<Column>()
      .box_it()
  }
}
fn main() {
  env_logger::init();
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
