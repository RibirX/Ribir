use ribir::prelude::*;

use super::common::{coming_soon, section_page};

pub fn page_concepts() -> Widget<'static> {
  section_page(
    "Concepts",
    "Core ideas and interactive explanations will land here.",
    coming_soon(
      "Concept notes are under construction.",
      "Short explainers for reactivity, fat objects, and declarative composition will be added \
       soon.",
    ),
  )
}
