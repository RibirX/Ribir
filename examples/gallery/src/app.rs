use ribir::prelude::*;
use smallvec::smallvec;

use crate::{sections::*, styles::*};

fn section_label(path: &str) -> &'static str {
  match path {
    "/" => "Showcase",
    "/widgets" => "Widgets",
    "/animations" => "Animations",
    "/concepts" => "Concepts",
    _ => "Showcase",
  }
}

fn gallery_breadcrumb(location: Stateful<Location>) -> Widget<'static> {
  fn_widget! {
    let palette = Palette::of(BuildCtx::get());

    @Flex {
      direction: Direction::Horizontal,
      margin: EdgeInsets::only_top(16.),
      align_items: Align::Center,
      item_gap: 4.,
      @TextButton {
        providers: [Provider::new(palette.on_surface())],
        on_tap: move |_| {
          let _ = $write(location).resolve_relative("/");
        },
        @ { "Gallery" }
      }
      @Icon {
        class: GALLERY_BREADCRUMB_SEPARATOR,
        @ { svg_registry::get_or_default("chevron_right") }
      }
      @TextButton {
        providers: [Provider::new(palette.on_surface())],
        @ { pipe!(section_label($read(location).path()).to_string()) }
      }
    }
  }
  .into_widget()
}

pub fn gallery() -> Widget<'static> {
  fn_widget! {
    let location = Location::state_of(BuildCtx::get());

    let expanded = Stateful::new(RailExpanded(false));
    let palette = Palette::of(BuildCtx::get());

    @Providers {
      providers: styles(),
      // `Expanded` only receives remaining space inside `Flex`.
      // Using `Row` here makes the content shell measure at its intrinsic width,
      // which can push the rounded shell flush against the right edge.
      @Flex {
        direction: Direction::Horizontal,
        clamp: BoxClamp::EXPAND_BOTH,
        align_items: Align::Stretch,

        @Providers {
          providers: smallvec![
             Provider::watcher(expanded.clone_watcher()),
             Provider::new(RailLabelPolicy::OnSelected),
             Provider::new(RailContentAlign(Align::Start)),
          ],
          @NavigationRail {
            selected: pipe!(CowArc::from($read(location).path().to_string())),
            on_custom: move |e: &mut RailSelectEvent| {
              let _ = $write(location).resolve_relative(&e.data().to);
            },

            @RailMenu {
              @TextButton {
                providers: [Provider::new(palette.on_surface_variant())],
                on_tap: move |_| $write(expanded).toggle(),
                @Icon { @ { svg_registry::get_or_default("menu") } }
              }
            }

            @RailItem {
              key: CowArc::from("/"),
              @ { svg_registry::get_or_default("home") }
              @ { "Showcase" }
            }
            @RailItem {
              key: CowArc::from("/widgets"),
              @ { svg_registry::get_or_default("widgets") }
              @ { "Widgets" }
            }
            @RailItem {
              key: CowArc::from("/animations"),
              @ { svg_registry::get_or_default("movie_filter") }
              @ { "Animations" }
            }
            @RailItem {
              key: CowArc::from("/concepts"),
              @ { svg_registry::get_or_default("auto_awesome_motion") }
              @ { "Concepts" }
            }

            @RailFooter {
              @TextButton {
                providers: [Provider::new(palette.on_surface_variant())],
                @Icon { @ { svg_registry::get_or_default("settings") } }
              }
            }
          }
        }

        @Expanded {
          @Flex {
            direction: Direction::Vertical,
            clamp: BoxClamp::EXPAND_BOTH,
            align_items: Align::Stretch,
            @gallery_breadcrumb(location)
            @Expanded {
              @Container {
                clamp: BoxClamp::EXPAND_BOTH,
                margin: EdgeInsets::new(16., 16., 16., 0.),
                class: GALLERY_CONTENT_SHELL,
                @Router {
                  @Route {
                    path: "/",
                    @ { page_showcase }
                  }
                  @Route {
                    path: "/widgets",
                    @ { page_widgets }
                  }
                  @Route {
                    path: "/animations",
                    @ { page_animations }
                  }
                  @Route {
                    path: "/concepts",
                    @ { page_concepts }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  .into_widget()
}
