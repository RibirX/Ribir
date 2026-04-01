use ribir::prelude::*;

use crate::styles::*;

pub fn section_page(
  title: &'static str, lead: &'static str, content: Widget<'static>,
) -> Widget<'static> {
  // `section_page` needs a flexible body area for the section content.
  // `Column` only performs sequential layout and doesn't allocate
  // remaining space to `Expanded` children, so use a vertical `Flex`
  // inside a bounded container.
  flex! {
    class: GALLERY_PAGE,
    direction: Direction::Vertical,
    clamp: BoxClamp::EXPAND_BOTH,
    align_items: Align::Stretch,
    @Column {
      class: GALLERY_PAGE_HEADER,
      @Text {
        class: GALLERY_PAGE_TITLE,
        text: title,
      }
      @Text {
        class: GALLERY_PAGE_LEAD,
        text: lead,
      }
    }
    @Expanded {
      @ { content }
    }
  }
  .into_widget()
}

pub fn coming_soon(title: &'static str, desc: &'static str) -> Widget<'static> {
  container! {
    clamp: BoxClamp::EXPAND_BOTH,
    class: GALLERY_STATUS_PANEL,
    @Flex {
      clamp: BoxClamp::EXPAND_BOTH,
      direction: Direction::Vertical,
      x: AnchorX::center(),
      y: AnchorY::center(),
      align_items: Align::Center,
      justify_content: JustifyContent::Center,
      item_gap: 6.,
      @Text {
        class: GALLERY_STATUS_BADGE,
        text: "COMING SOON",
      }
      @Text {
        class: GALLERY_STATUS_TITLE,
        text: title,
      }
      @Text {
        class: GALLERY_STATUS_BODY,
        text: desc,
      }
    }
  }
  .into_widget()
}
