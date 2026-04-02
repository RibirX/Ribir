use ribir::prelude::*;
use smallvec::smallvec;

use super::common::section_page;
use crate::styles::*;

// --- State ---

#[derive(Clone, Copy, PartialEq, Eq)]
struct RailStructureState {
  show_fab: bool,
  show_menu: bool,
  section_show_collapsed: bool,
}

impl Default for RailStructureState {
  fn default() -> Self { Self { show_fab: true, show_menu: true, section_show_collapsed: true } }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct RailControlState {
  expanded: bool,
  label_policy: RailLabelPolicy,
  content_align: RailContentAlign,
}

#[derive(Clone, Copy, PartialEq)]
struct MailState {
  selected_idx: usize,
  badge_count: f32,
}

impl Default for MailState {
  fn default() -> Self { Self { selected_idx: 0, badge_count: 12. } }
}

impl Compose for RailStructureState {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    let controls = Stateful::new(RailControlState::default());
    let mail = Stateful::new(MailState::default());
    fn_widget! {
      @Providers {
        providers: smallvec![
          Provider::writer(this.clone_writer(), None),
          Provider::writer(mail.clone_writer(), None),
        ],
        @Flex {
          clamp: BoxClamp::EXPAND_BOTH,
          direction: Direction::Horizontal,
          align_items: Align::Stretch,
          item_gap: 12.,
          @Expanded {
            @ { controls.clone_writer() }
          }
          @Scrollbar {
            clamp: BoxClamp::fixed_width(220.),
            scrollable: Scrollable::Y,
            @Flex {
              class: GALLERY_RAIL_CONTROLS_PANEL,
              direction: Direction::Vertical,
              item_gap: 12.,
              align_items: Align::Stretch,

              @Switch {
                checked: TwoWay::new(part_writer!(&mut controls.expanded)),
                @Leading::new("Expanded")
              }
              @Text {
                margin: EdgeInsets::only_top(4.),
                text: "Label policy",
                text_overflow: TextOverflow::AutoWrap,
              }
              @label_policy_selector(controls.clone_writer())
              @Switch {
                checked: TwoWay::new(part_writer!(&mut this.section_show_collapsed)),
                @Leading::new("Section in collapsed")
              }
              @Text {
                margin: EdgeInsets::only_top(4.),
                text: "Content align",
                text_overflow: TextOverflow::AutoWrap,
              }
              @content_align_selector(controls.clone_writer())
              @Switch {
                checked: TwoWay::new(part_writer!(&mut this.show_fab)),
                @Leading::new("Show FAB")
              }
              @Switch {
                checked: TwoWay::new(part_writer!(&mut this.show_menu)),
                @Leading::new("Show menu")
              }
              @Text {
                margin: EdgeInsets::only_top(4.),
                text: "Badge count",
                text_overflow: TextOverflow::AutoWrap,
              }
              @Slider {
                value: TwoWay::new(part_writer!(&mut mail.badge_count)),
                min: 0.,
                max: 99.,
                divisions: Some(10),
              }
            }
          }
        }
      }
    }
    .into_widget()
  }
}

impl Compose for RailControlState {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let structure = Provider::state_of::<Stateful<RailStructureState>>(BuildCtx::get())
        .expect("RailStructureState provider must be in scope")
        .clone_writer();
      let mail = Provider::state_of::<Stateful<MailState>>(BuildCtx::get())
        .expect("MailState provider must be in scope")
        .clone_writer();

      @Providers {
        providers: smallvec![
          Provider::watcher(this.part_watcher(|state| PartRef::from_value(RailExpanded(
            state.expanded,
          )))),
          Provider::watcher(this.part_watcher(|state| PartRef::from_value(state.label_policy))),
          Provider::watcher(this.part_watcher(|state| PartRef::from_value(state.content_align))),
        ],
        @Flex {
          clamp: BoxClamp::EXPAND_BOTH,
          direction: Direction::Horizontal,
          align_items: Align::Stretch,
          item_gap: 12.,
          @ {
            distinct_pipe!(*$read(structure)).map(move |structure| {
              fn_widget! {
                let action = structure.show_fab.then(|| {
                  @RailFabAction {
                    @Icon { @ { svg_registry::get_or_default("edit") } }
                    @ { "Compose" }
                  }
                });
                let menu = structure.show_menu.then(|| {
                  @RailMenu {
                    @TextButton {
                      on_tap: move |_| $write(this).expanded ^= true,
                      @Icon { @ { svg_registry::get_or_default("menu") } }
                    }
                  }
                });

                @NavigationRail {
                  selected: distinct_pipe!(
                    DESTINATIONS[$read(mail).selected_idx].1
                  ),
                  on_custom: move |e: &mut RailSelectEvent| {
                    let key = e.data().to.clone();
                    if let Some(idx) = destination_idx(&key) {
                      $write(mail).selected_idx = idx;
                    }
                  },

                  @ { menu }
                  @ { action }
                  @RailSection {
                    show_collapsed: structure.section_show_collapsed,
                    @ { MAIL_SECTION_LABEL }
                  }
                  @mail_inbox_item($writer(mail))
                  @mail_item(1)
                  @mail_item(2)
                  @mail_item(3)
                }
              }
            })
          }
          @Expanded {
            @Flex {
              class: GALLERY_RAIL_CONTROLS_PANEL,
              clamp: BoxClamp::EXPAND_BOTH,
              direction: Direction::Vertical,
              align_items: Align::Stretch,
              item_gap: 12.,
              @Text {
                text: pipe!(if $read(structure).section_show_collapsed {
                  "Section stays visible"
                } else {
                  "Section hides when collapsed"
                }),
                text_style: TypographyTheme::of(BuildCtx::get()).title_medium.text.clone(),
              }
              @Text {
                text: pipe!(if $read(structure).section_show_collapsed {
                  "Collapsed mode keeps the Mail section marker so the group stays discoverable."
                } else {
                  "Collapsed mode removes the Mail section marker for a cleaner, denser rail."
                }),
                text_overflow: TextOverflow::AutoWrap,
                foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
              }
              @Text {
                text: pipe!(if $read(this).expanded {
                  "Mode: expanded"
                } else {
                  "Mode: collapsed"
                }),
                foreground: Palette::of(BuildCtx::get()).secondary(),
              }
              @Text {
                text: pipe!(if $read(structure).section_show_collapsed {
                  "Collapsed section: shown"
                } else {
                  "Collapsed section: hidden"
                }),
                foreground: Palette::of(BuildCtx::get()).secondary(),
              }
              @Text {
                text: distinct_pipe! {
                  let label = DESTINATIONS[$read(mail).selected_idx].2;
                  format!("Selected: {label}")
                },
                text_style: TypographyTheme::of(BuildCtx::get()).label_large.text.clone(),
              }
              @Expanded {
                @ { distinct_pipe!($read(mail).selected_idx).map(mail_content) }
              }
            }
          }
        }
      }
    }
    .into_widget()
  }
}

const DESTINATIONS: &[(&str, &str, &str)] = &[
  ("inbox", "inbox", "Inbox"),
  ("send", "sent", "Sent"),
  ("star", "starred", "Starred"),
  ("report", "spam", "Spam"),
];
const LABEL_POLICY_OPTIONS: &[(&str, RailLabelPolicy)] = &[
  ("None", RailLabelPolicy::None),
  ("Selected", RailLabelPolicy::OnSelected),
  ("Always", RailLabelPolicy::Always),
];
const CONTENT_ALIGN_OPTIONS: &[(&str, RailContentAlign)] = &[
  ("Top", RailContentAlign(Align::Start)),
  ("Center", RailContentAlign(Align::Center)),
  ("Bottom", RailContentAlign(Align::End)),
];
const MAIL_SECTION_LABEL: &str = "Mail";

fn destination_idx(key: &str) -> Option<usize> {
  DESTINATIONS
    .iter()
    .position(|(_, destination_key, _)| *destination_key == key)
}

// --- Mail content area ---

fn mail_content(selected_idx: usize) -> Widget<'static> {
  let (icon, _, label) = DESTINATIONS[selected_idx];
  flex! {
    clamp: BoxClamp::EXPAND_BOTH,
    direction: Direction::Vertical,
    align_items: Align::Center,
    justify_content: JustifyContent::Center,
    item_gap: 12.,
    @Icon {
      @ { svg_registry::get_or_default(icon) }
    }
    @Text {
      text: label,
      text_style: TypographyTheme::of(BuildCtx::get()).title_medium.text.clone(),
    }
    @Text {
      text: "Selected destination",
      text_align: TextAlign::Center,
      text_overflow: TextOverflow::AutoWrap,
      foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
    }
  }
  .into_widget()
}

// --- Mail app sandbox (providers wrapper + NavigationRail) ---

fn mail_inbox_item(state: Stateful<MailState>) -> PairOf<'static, RailItem> {
  let (icon, key, label) = DESTINATIONS[0];
  rdl! {
    @RailItem {
      key: key,
      @ { svg_registry::get_or_default(icon) }
      @ { label }
      @NumBadge {
        count: distinct_pipe! {
          let count = $read(state).badge_count.round() as u32;
          (count > 0).then_some(count)
        }
      }
    }
  }
  .r_into()
}

fn mail_item(idx: usize) -> PairOf<'static, RailItem> {
  let (icon, key, label) = DESTINATIONS[idx];
  rdl! {
    @RailItem {
      key: key,
      @ { svg_registry::get_or_default(icon) }
      @ { label }
    }
  }
  .r_into()
}

fn choice_selector<T, S>(selected: S, options: &'static [(&'static str, T)]) -> Widget<'static>
where
  T: Copy + PartialEq + 'static,
  S: StateWriter<Value = T> + 'static,
{
  fn_widget! {
    let button = move |label: &'static str, value: T| {
      let selected = selected.clone_writer();
      pipe!(*$read(selected) == value).map(move |active| {
        if active {
          filled_button! {
            on_tap: move |_| *$write(selected) = value,
            @ { label }
          }
          .into_widget()
        } else {
          text_button! {
            on_tap: move |_| *$write(selected) = value,
            @ { label }
          }
          .into_widget()
        }
      })
    };

    @Flex {
      direction: Direction::Horizontal,
      wrap: true,
      item_gap: 4.,
      line_gap: 4.,
      @ { options.iter().copied().map(move |(label, value)| button(label, value)) }
    }
  }
  .into_widget()
}

fn label_policy_selector(state: Stateful<RailControlState>) -> Widget<'static> {
  choice_selector(part_writer!(&mut state.label_policy), LABEL_POLICY_OPTIONS)
}

fn content_align_selector(state: Stateful<RailControlState>) -> Widget<'static> {
  choice_selector(part_writer!(&mut state.content_align), CONTENT_ALIGN_OPTIONS)
}

// --- Page ---

pub fn page_navigation_rail() -> Widget<'static> {
  section_page(
    "Navigation Rail",
    "Side navigation for medium and large screens. Try the controls on the right.",
    Stateful::new(RailStructureState::default()).into_widget(),
  )
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;

  use super::*;

  const COLLAPSED_INDICATOR_SIZE: Size = Size::new(56., 32.);

  fn count_collapsed_indicator_paths(commands: &[PaintCommand], indicator_color: Color) -> usize {
    commands
      .iter()
      .map(|command| match command {
        PaintCommand::Path(path)
          if matches!(
            &path.action,
            PaintPathAction::Paint {
              painting_style: PaintingStyle::Fill,
              brush: CommandBrush::Color(color),
            } if *color == indicator_color
          ) && (path.paint_bounds.width() - COLLAPSED_INDICATOR_SIZE.width).abs() < 0.5
            && (path.paint_bounds.height() - COLLAPSED_INDICATOR_SIZE.height).abs() < 0.5 =>
        {
          1
        }
        PaintCommand::Bundle { cmds, .. } => count_collapsed_indicator_paths(cmds, indicator_color),
        _ => 0,
      })
      .sum()
  }

  #[test]
  fn toggling_menu_and_fab_rebuilds_mail_rail_structure() {
    reset_test_env!();
    AppCtx::set_app_theme(material::purple::light());

    let indicator_color = material::purple::light()
      .palette
      .secondary_container();

    let structure = Stateful::new(RailStructureState::default());
    let controls = Stateful::new(RailControlState::default());
    let mail = Stateful::new(MailState::default());
    let scope_structure = structure.clone_writer();
    let scope_controls = controls.clone_writer();
    let scope_mail = mail.clone_writer();
    let mut wnd = TestWindow::new_with_size(
      move || {
        let structure = scope_structure.clone_writer();
        let controls = scope_controls.clone_writer();
        let mail = scope_mail.clone_writer();
        providers! {
          providers: smallvec![
            Provider::writer(structure, None),
            Provider::writer(mail, None),
          ],
          @ { controls }
        }
        .into_widget()
      },
      Size::new(360., 720.),
    );

    wnd.draw_frame();
    let _ = wnd.take_last_frame();

    {
      let mut state = structure.write();
      state.show_menu = false;
      state.show_fab = false;
    }
    wnd.draw_frame();
    let frame = wnd
      .take_last_frame()
      .expect("rebuild frame should be painted");
    assert_eq!(
      count_collapsed_indicator_paths(&frame.commands, indicator_color),
      1,
      "structural rebuild should not paint transient indicators for unselected items"
    );

    {
      let mut state = structure.write();
      state.show_menu = true;
      state.show_fab = true;
    }
    wnd.draw_frame();
    let frame = wnd
      .take_last_frame()
      .expect("second rebuild frame should be painted");
    assert_eq!(
      count_collapsed_indicator_paths(&frame.commands, indicator_color),
      1,
      "restoring structural slots should still keep only the selected indicator visible"
    );
  }

  #[test]
  fn turning_fab_off_then_expanding_navigation_rail_does_not_panic() {
    reset_test_env!();

    let structure = Stateful::new(RailStructureState::default());
    let controls = Stateful::new(RailControlState::default());
    let mail = Stateful::new(MailState::default());
    let root_structure = structure.clone_writer();
    let root_controls = controls.clone_writer();
    let root_mail = mail.clone_writer();
    let wnd = TestWindow::new_with_size(
      move || {
        let structure = root_structure.clone_writer();
        let controls = root_controls.clone_writer();
        let mail = root_mail.clone_writer();
        providers! {
          providers: smallvec![
            Provider::writer(structure, None),
            Provider::writer(mail, None),
          ],
          @ { controls }
        }
        .into_widget()
      },
      Size::new(1080., 840.),
    );

    wnd.draw_frame();

    {
      structure.write().show_fab = false;
      controls.write().expanded = true;
    }

    wnd.draw_frame();
  }

  #[test]
  fn changing_content_align_to_bottom_does_not_panic() {
    reset_test_env!();

    let structure = Stateful::new(RailStructureState::default());
    let controls = Stateful::new(RailControlState::default());
    let mail = Stateful::new(MailState::default());
    let root_structure = structure.clone_writer();
    let root_controls = controls.clone_writer();
    let root_mail = mail.clone_writer();
    let wnd = TestWindow::new_with_size(
      move || {
        let structure = root_structure.clone_writer();
        let controls = root_controls.clone_writer();
        let mail = root_mail.clone_writer();
        providers! {
          providers: smallvec![
            Provider::writer(structure, None),
            Provider::writer(mail, None),
          ],
          @ { controls }
        }
        .into_widget()
      },
      Size::new(1080., 840.),
    );

    wnd.draw_frame();

    controls.write().content_align = RailContentAlign(Align::End);

    wnd.draw_frame();
  }

  #[test]
  fn toggling_collapsed_section_visibility_does_not_panic() {
    reset_test_env!();

    let structure = Stateful::new(RailStructureState::default());
    let controls = Stateful::new(RailControlState::default());
    let mail = Stateful::new(MailState::default());
    let root_structure = structure.clone_writer();
    let root_controls = controls.clone_writer();
    let root_mail = mail.clone_writer();
    let wnd = TestWindow::new_with_size(
      move || {
        let structure = root_structure.clone_writer();
        let controls = root_controls.clone_writer();
        let mail = root_mail.clone_writer();
        providers! {
          providers: smallvec![
            Provider::writer(structure, None),
            Provider::writer(mail, None),
          ],
          @ { controls }
        }
        .into_widget()
      },
      Size::new(1080., 840.),
    );

    wnd.draw_frame();

    structure.write().section_show_collapsed = false;

    wnd.draw_frame();
  }
}
