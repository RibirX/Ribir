use ribir::prelude::*;

use super::common::{coming_soon, section_page};

pub fn page_animations() -> Widget<'static> {
  section_page(
    "Animations",
    "Motion studies and transition demos are being prepared.",
    coming_soon(
      "Animation demos are coming soon.",
      "This page will collect transitions, motion patterns, and small interactive studies.",
    ),
  )
}
