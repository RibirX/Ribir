use ribir::prelude::*;

use super::{common::section_page, navigation_rail::page_navigation_rail};
use crate::styles::*;

const NAVIGATION_RAIL_PATH: &str = "/widgets/navigation-rail";

fn navigation_rail_card() -> Widget<'static> {
  fn_widget! {
    let location = Location::state_of(BuildCtx::get());

    @Container {
      clamp: BoxClamp::EXPAND_BOTH,
      class: GALLERY_STATUS_PANEL,
      @Flex {
        clamp: BoxClamp::EXPAND_BOTH,
        direction: Direction::Vertical,
        align_items: Align::Start,
        justify_content: JustifyContent::Center,
        item_gap: 16.,
        @Text {
          class: GALLERY_STATUS_BADGE,
          text: "WIDGET DEMO",
        }
        @Text {
          class: GALLERY_STATUS_TITLE,
          text: "Navigation Rail",
        }
        @Text {
          class: GALLERY_STATUS_BODY,
          text: "Explore a live Material 3 navigation rail sandbox with selection, badges, menu, sections, and expanded layout controls.",
        }
        @FilledButton {
          on_tap: move |_| {
            let _ = $write(location).resolve_relative(NAVIGATION_RAIL_PATH);
          },
          @Icon {
            @ { svg_registry::get_or_default("navigation") }
          }
          @ { "Open demo" }
        }
      }
    }
  }
  .into_widget()
}

fn page_widgets_home() -> Widget<'static> {
  section_page(
    "Widgets",
    "Browse interactive widget demos and component-focused sandboxes.",
    navigation_rail_card(),
  )
}

pub fn page_widgets() -> Widget<'static> {
  router! {
    @Route {
      path: "/",
      @ { page_widgets_home }
    }
    @Route {
      path: "/navigation-rail",
      @ { page_navigation_rail }
    }
  }
  .into_widget()
}
