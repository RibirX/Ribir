use ribir::prelude::*;
use smallvec::smallvec;

pub fn gallery() -> Widget<'static> {
  let expanded = Stateful::new(RailExpanded(true));

  providers! {
    providers: smallvec![
      Provider::writer(expanded.clone_writer(), None),
      Provider::new(RailLabelPolicy::OnSelected),
      Provider::new(RailContentAlign(Align::Start)),
    ],
    @Row {
      height: Measure::Unit(1.),
      @NavigationRail {
        height: Measure::Unit(1.),
        selected: "todos",
        @RailMenu {
          @TextButton {
            on_tap: move |_| $write(expanded).toggle(),
            @Icon { @ { svg_registry::get_or_default("menu") } }
          }
        }
        @RailFabAction {
          @Icon { @ { svg_registry::get_or_default("search") } }
          @{ "Search" }
        }
        @RailItem {
          key: "widgets",
          @ { svg_registry::get_or_default("dashboard") }
          @ { "Widgets" }
        }
        @RailItem {
          key: "layout",
          @ { svg_registry::get_or_default("dashboard") }
          @ { "Layout" }
        }
        @RailItem {
          key: "animation",
          @ { svg_registry::get_or_default("animation") }
          @ { "Animation" }
        }

        @RailFooter {
          @Flex {
            direction: Direction::Horizontal,
            item_gap: 8.,
            align_items: Align::Center,
            @Icon { @ { svg_registry::get_or_default("light_mode") } }
            @Text { text: "Light Mode" }
          }
        }
      }
      @Expanded {
        @H1 {
          y: AnchorY::center(),
          text_align: TextAlign::Center,
          text: "Content Area, Coming Soon..."
        }
      }
    }
  }
  .into_widget()
}
