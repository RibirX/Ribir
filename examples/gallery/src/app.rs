use ribir::prelude::*;

pub fn gallery() -> Widget<'static> {
  icon! {
    v_align: VAlign::Center,
    h_align: HAlign::Center,
    text_line_height: 128.,
    @ asset!("../assets/logo.svg", "svg")
  }
  .into_widget()
}
