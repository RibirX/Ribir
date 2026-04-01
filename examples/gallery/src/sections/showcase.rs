use ribir::prelude::*;

use super::common::{coming_soon, section_page};

pub fn page_showcase() -> Widget<'static> {
  section_page(
    "Showcase",
    "Practical example walkthroughs will live here.",
    coming_soon(
      "Showcase examples are under construction.",
      "Interactive demos and deeper source-guided tours will be added soon.",
    ),
  )
}
