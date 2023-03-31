use ribir::prelude::*;
fn main() {
  app::run(widget! {
    states { cnt: Stateful::new(0) }
    Column {
      h_align: HAlign::Center,
      align_items: Align::Center,
      FilledButton { on_tap: move |_| *cnt += 1, Label::new("Add") }
      H1 { text: cnt.to_string() }
      FilledButton { on_tap: move |_| *cnt += -1, Label::new("Sub") }
    }
  });
}
