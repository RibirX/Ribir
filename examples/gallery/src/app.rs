use ribir::prelude::*;
use smallvec::smallvec;

use crate::{sections::*, styles::*};

const SHOWCASE_PATH: &str = "/";
const WIDGETS_PATH: &str = "/widgets";
const WIDGETS_ROUTE: &str = "/widgets/*";
const NAVIGATION_RAIL_PATH: &str = "/widgets/navigation-rail";
const LEGACY_NAVIGATION_RAIL_PATH: &str = "/navigation-rail";
const ANIMATIONS_PATH: &str = "/animations";
const CONCEPTS_PATH: &str = "/concepts";

const SHOWCASE_BREADCRUMB: &[(&str, &str)] = &[(SHOWCASE_PATH, "Showcase")];
const WIDGETS_BREADCRUMB: &[(&str, &str)] = &[(WIDGETS_PATH, "Widgets")];
const NAVIGATION_RAIL_BREADCRUMB: &[(&str, &str)] =
  &[(WIDGETS_PATH, "Widgets"), (NAVIGATION_RAIL_PATH, "Navigation Rail")];
const ANIMATIONS_BREADCRUMB: &[(&str, &str)] = &[(ANIMATIONS_PATH, "Animations")];
const CONCEPTS_BREADCRUMB: &[(&str, &str)] = &[(CONCEPTS_PATH, "Concepts")];

fn top_level_section_path(path: &str) -> &'static str {
  if path == WIDGETS_PATH || path == LEGACY_NAVIGATION_RAIL_PATH || path.starts_with("/widgets/") {
    WIDGETS_PATH
  } else {
    match path {
      SHOWCASE_PATH => SHOWCASE_PATH,
      ANIMATIONS_PATH => ANIMATIONS_PATH,
      CONCEPTS_PATH => CONCEPTS_PATH,
      _ => SHOWCASE_PATH,
    }
  }
}

fn breadcrumb_trail(path: &str) -> &'static [(&'static str, &'static str)] {
  match path {
    WIDGETS_PATH => WIDGETS_BREADCRUMB,
    NAVIGATION_RAIL_PATH | LEGACY_NAVIGATION_RAIL_PATH => NAVIGATION_RAIL_BREADCRUMB,
    ANIMATIONS_PATH => ANIMATIONS_BREADCRUMB,
    CONCEPTS_PATH => CONCEPTS_BREADCRUMB,
    _ => SHOWCASE_BREADCRUMB,
  }
}

fn breadcrumb_separator() -> Widget<'static> {
  icon! {
    class: GALLERY_BREADCRUMB_SEPARATOR,
    @ { svg_registry::get_or_default("chevron_right") }
  }
  .into_widget()
}

fn breadcrumb_link(
  label: &'static str, target: &'static str, location: Stateful<Location>,
) -> Widget<'static> {
  text_button! {
    providers: [Provider::new(Palette::of(BuildCtx::get()).on_surface())],
    on_tap: move |_| {
      let _ = $write(location).resolve_relative(target);
    },
    @ { label }
  }
  .into_widget()
}

fn breadcrumb_current(label: &'static str) -> Widget<'static> {
  text! {
    text: label,
    foreground: Palette::of(BuildCtx::get()).on_surface(),
  }
  .into_widget()
}

fn build_breadcrumb(path: &str, location: Stateful<Location>) -> Widget<'static> {
  let trail = breadcrumb_trail(path);
  let mut crumbs = Vec::with_capacity(trail.len() * 2 + 1);
  crumbs.push(breadcrumb_link("Gallery", SHOWCASE_PATH, location.clone_writer()));

  for (idx, &(target, label)) in trail.iter().enumerate() {
    crumbs.push(breadcrumb_separator());
    crumbs.push(if idx + 1 == trail.len() {
      breadcrumb_current(label)
    } else {
      breadcrumb_link(label, target, location.clone_writer())
    });
  }

  flex! {
    direction: Direction::Horizontal,
    margin: EdgeInsets::only_top(16.),
    align_items: Align::Center,
    item_gap: 4.,
    @ { crumbs }
  }
  .into_widget()
}

fn redirect_to(target: &'static str) -> Widget<'static> {
  fn_widget! {
    let location = Location::state_of(BuildCtx::get());
    @Void {
      on_mounted: move |_| {
        let _ = $write(location).resolve_relative(target);
      },
    }
  }
  .into_widget()
}

fn page_navigation_rail_redirect() -> Widget<'static> { redirect_to(NAVIGATION_RAIL_PATH) }

fn gallery_breadcrumb(location: Stateful<Location>) -> Widget<'static> {
  fn_widget! {
    @ {
      pipe!($read(location).path().to_string())
        .map(move |path| build_breadcrumb(path.as_str(), location.clone_writer()))
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
            selected: distinct_pipe!(top_level_section_path($read(location).path())),
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
              key: SHOWCASE_PATH,
              @ { svg_registry::get_or_default("home") }
              @ { "Showcase" }
            }
            @RailItem {
              key: WIDGETS_PATH,
              @ { svg_registry::get_or_default("widgets") }
              @ { "Widgets" }
            }
            @RailItem {
              key: ANIMATIONS_PATH,
              @ { svg_registry::get_or_default("movie_filter") }
              @ { "Animations" }
            }
            @RailItem {
              key: CONCEPTS_PATH,
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
                    path: SHOWCASE_PATH,
                    @ { page_showcase }
                  }
                  @Route {
                    path: WIDGETS_ROUTE,
                    @ { page_widgets }
                  }
                  @Route {
                    path: ANIMATIONS_PATH,
                    @ { page_animations }
                  }
                  @Route {
                    path: CONCEPTS_PATH,
                    @ { page_concepts }
                  }
                  @Route {
                    path: LEGACY_NAVIGATION_RAIL_PATH,
                    @ { page_navigation_rail_redirect }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nested_widget_routes_keep_widgets_selected() {
    assert_eq!(top_level_section_path(SHOWCASE_PATH), SHOWCASE_PATH);
    assert_eq!(top_level_section_path(WIDGETS_PATH), WIDGETS_PATH);
    assert_eq!(top_level_section_path(NAVIGATION_RAIL_PATH), WIDGETS_PATH);
    assert_eq!(top_level_section_path(LEGACY_NAVIGATION_RAIL_PATH), WIDGETS_PATH);
  }

  #[test]
  fn nested_widget_routes_build_expected_breadcrumbs() {
    assert_eq!(breadcrumb_trail(WIDGETS_PATH), WIDGETS_BREADCRUMB);
    assert_eq!(breadcrumb_trail(NAVIGATION_RAIL_PATH), NAVIGATION_RAIL_BREADCRUMB,);
  }
}
