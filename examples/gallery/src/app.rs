use ribir::prelude::*;

pub fn gallery() -> Widget<'static> {
  icon! {
    x: AnchorX::center(),
    y: AnchorY::center(),
    text_line_height: 128.,
    @ asset!("../assets/logo.svg", "svg")
  }
  .into_widget()
}
