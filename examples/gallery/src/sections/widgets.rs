use ribir::prelude::*;

use super::common::{coming_soon, section_page};

pub fn page_widgets() -> Widget<'static> {
  section_page(
    "Widgets",
    "A compact widget reference is on the way.",
    coming_soon(
      "Widget previews are under construction.",
      "Live component states, code snippets, and categorised references will appear here soon.",
    ),
  )
}
