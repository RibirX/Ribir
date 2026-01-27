use ribir::prelude::*;
use smallvec::smallvec;

pub fn gallery() -> Widget<'static> {
  let expanded = Stateful::new(RailExpanded(true));

  providers! {
    providers: smallvec![
      Provider::writer(expanded.clone_writer(), None),
      Provider::new(RailLabelPolicy::Always),
      Provider::new(RailContentAlign(Align::Start)),
    ],
    @Row {
      @NavigationRail {
        selected: "todos",
        @RailMenu {
          @FatObj {
            on_tap: move |_| $write(expanded).toggle(),
            @ { svg_registry::get_or_default("menu") }
          }
        }
        @RailSection { @ { "Demos" } }
        @RailItem {
          key: "todos",
          @ { svg_registry::get_or_default("checklist") }
          @ { "Todos" }
        }
        @RailItem {
          key: "messages",
          @ { svg_registry::get_or_default("chat") }
          @ { "Messages" }
        }
        @RailItem {
          key: "wordle",
          @ { svg_registry::get_or_default("grid_view") }
          @ { "Wordle" }
        }
        @RailItem {
          key: "pomodoro",
          @ { svg_registry::get_or_default("timer") }
          @ { "Pomodoro" }
        }

        @RailSection { @ { "Widgets" } }
        @RailItem {
          key: "button",
          @ { svg_registry::get_or_default("smart_button") }
          @ { "Button" }
        }
        @RailItem {
          key: "input",
          @ { svg_registry::get_or_default("input") }
          @ { "Input" }
        }

        @RailSection { @ { "Concepts" } }
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
        @Icon {
          text_line_height: 128.,
          @ asset!("../assets/logo.svg", "svg")
        }
      }
    }
  }
  .into_widget()
}
