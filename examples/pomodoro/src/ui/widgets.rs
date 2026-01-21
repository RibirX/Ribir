use std::time::Duration;

use ribir::prelude::*;

use crate::{
  pomodoro::{Pomodoro, PomodoroState, UPDATE_INTERVAL},
  ui::{APP_ICON, PomodoroPage, UiState, styles::*},
};

fn duration_to_mmss(d: Duration) -> String {
  let mut s = d.as_secs();
  let m = s / 60;
  s -= m * 60;
  format!("{:02}:{:02}", m, s)
}

fn run_button() -> Widget<'static> {
  fn_widget! {
    let pomodoro = Provider::writer_of::<Pomodoro>(BuildCtx::get()).unwrap();
    @Icon {
      cursor: CursorIcon::Pointer,
      on_tap: {
        let pomodoro = pomodoro.clone_writer();
        move |_| {
          let v = $read(pomodoro).is_running();
          if v {
            $write(pomodoro).pause();
          } else {
            Pomodoro::run(&pomodoro, UPDATE_INTERVAL);
          }
        }
      },
      @{
        pipe!($read(pomodoro).is_running())
          .transform(|obs| obs.distinct_until_changed())
          .map(move |running| {
            if running {
              svg_registry::get("pause")
            } else {
              svg_registry::get("play")
            }
          })
      }
    }
  }
  .into_widget()
}

fn mode_icon() -> Widget<'static> {
  fn_widget! {
    let ui_state = Provider::writer_of::<UiState>(BuildCtx::get()).unwrap();
    @Icon {
      cursor: CursorIcon::Pointer,
      on_tap: move |_| {
        let v = $read(ui_state).in_mini();
        if v {
          $write(ui_state).current_page = PomodoroPage::Main;
        } else {
          $write(ui_state).current_page = PomodoroPage::Mini;
        }
      },
      @{
        pipe!($read(ui_state).in_mini())
          .transform(|obs| obs.distinct_until_changed())
          .map(move |in_mini| {
            if in_mini {
              svg_registry::get("full")
            } else {
              svg_registry::get("mini")
            }
          })
      }
    }
  }
  .into_widget()
}

fn keep_icon() -> Widget<'static> {
  fn_widget! {
    let ui_state = Provider::writer_of::<UiState>(BuildCtx::get()).unwrap();
    @Icon {
      cursor: CursorIcon::Pointer,
      on_tap: move |e| {
        let keep_on_top = !$read(ui_state).keep_on_top;
        let level = if keep_on_top {
          WindowLevel::AlwaysOnTop
        } else {
          WindowLevel::Normal
        };
        e.window().set_window_level(level);
        $write(ui_state).keep_on_top = keep_on_top;
      },
      @{
        pipe!($read(ui_state).keep_on_top)
          .transform(|obs| obs.distinct_until_changed())
          .map(move |always_on_top| {
            if always_on_top {
              svg_registry::get("pin")
            } else {
              svg_registry::get("pin_off")
            }
          })
      }
    }
  }
  .into_widget()
}

#[declare]
pub(crate) struct WindowBar {}

impl Compose for WindowBar {
  fn compose(_state: impl StateWriter<Value = Self>) -> Widget<'static>
  where
    Self: Sized,
  {
    fn_widget! {
      let ui_state = Provider::watcher_of::<UiState>(BuildCtx::get()).unwrap();
      @Stack {
        class: pipe!($read(ui_state).in_mini())
          .map(|in_mini| if in_mini { MINI_WINDOW_BAR } else { WINDOW_BAR }),
        @PointerSelectRegion {
          on_custom: move |e: &mut PointerSelectEvent| {
            if let PointerSelectData::Move{ from, to } |
              PointerSelectData::End { from, to } = e.data() {
                e.window().set_position(e.window().position() + (*to - *from));
            }
          },
          @Container {
            @Text {
              x: AnchorX::center(),
              y: AnchorY::center(),
              visible: pipe!($read(ui_state).current_page != PomodoroPage::Mini),
              text: "Pomodoro",
            }
          }
        }

        @FatObj {
          x: AnchorX::left(),
          y: AnchorY::center(),
          @ { keep_icon() }
        }
        @Row {
          x: AnchorX::right(),
          y: AnchorY::center(),
          @Icon {
            cursor: CursorIcon::Pointer,
            on_tap: move |e| {
              e.window().shell_wnd().borrow_mut().set_minimized(true);
            },
            @{ svg_registry::get_or_default("minimize") }
          }
          @ { mode_icon() }
          @Icon {
            cursor: CursorIcon::Pointer,
            on_tap: move |e| e.window().close(),
            @{ svg_registry::get_or_default("close") }
          }
        }
      }
    }
    .into_widget()
  }
}

/// Main working pane displaying the timer, controls, and volume slider
pub(crate) fn main_page() -> Widget<'static> {
  fn_widget!{
    let pomodoro = Provider::writer_of::<Pomodoro>(BuildCtx::get()).unwrap();
    @Column {
      align_items: Align::Center,
      x: AnchorX::center(),
      @Container {
        margin: EdgeInsets::vertical(26.),
        size: Size::new(256., 256.),
        @Stack {
          @Column {
            x: AnchorX::center(),
            y: AnchorY::center(),
            align_items: Align::Center,
            @H1 {
              text: @pipe!($read(pomodoro).current_remaining)
                .map(duration_to_mmss)
            }
            @H4 {
              text: @pipe!($read(pomodoro).state).map(|s| format!("{:?}", s))
            }
          }
          @FittedBox {
            box_fit: BoxFit::Cover,
            @SpinnerProgress {
              class: CURRENT,
              x: AnchorX::center(),
              y: AnchorY::center(),
              value: pipe!(1. - $read(pomodoro).state_progress())
            }
          }
        }
      }

      @Icon {
        text_line_height: 64.,
        @ { run_button() }
      }

      @Flex {
        clamp: BoxClamp::fixed_size(Size::new(FULL_WIDTH, 80.)),
        align_items: Align::Center,
        padding: EdgeInsets::horizontal(10.),
        @Column {
          align_items: Align::Center,
          @Text { text: pipe!(($read(pomodoro).rounds, $read(pomodoro).config.cycles)). map(|(rounds, cycles)| format!("({}/{})", rounds, cycles))}
          @Text {
            cursor: CursorIcon::Pointer,
            on_tap: move |_| $write(pomodoro).reset(),
            text: "Reset"
          }
        }
        @Icon {
          margin: EdgeInsets::only_left(10.),
          text_line_height: 24.,
          cursor: CursorIcon::Pointer,
          on_tap: move |_| {
            let v = $read(pomodoro).volume;
            if v > 0. {
              $write(pomodoro).volume = 0.;
            } else {
              $write(pomodoro).volume = 1.;
            }
          },
          @ {
            pipe!($read(pomodoro).volume <= 0.)
            .transform(|obs| obs.distinct_until_changed())
            .map(move |muted| {
                if muted {
                svg_registry::get("volume_off")
                } else {
                svg_registry::get("volume_up")
                }
            })
          }
        }
        @Expanded {
          flex: 1.,
          @Slider {
            min: 0.0,
            max: 100.0,
            value: $read(pomodoro).volume * 100.0,
            on_custom: move |e: &mut SliderChangedEvent| {
              $write(pomodoro).volume = e.data().to / 100.;
            }
          }
        }
        @Icon {
          text_line_height: 32.,
          margin: EdgeInsets::only_left(10.),
          cursor: CursorIcon::Pointer,
          on_tap: move |_| $write(pomodoro).next_state(),
          tooltips: "skip",
          @{ svg_registry::get("skip_next").unwrap() }
        }
      }
    }
  }.into_widget()
}

/// About page widget displaying app information and icon
fn about() -> Widget<'static> {
  fn_widget! {
    @Container {
      clamp: BoxClamp::EXPAND_BOTH,
      @Column {
        y: AnchorY::center(),
        x: AnchorX::center(),
        align_items: Align::Center,
        @H4 { text: "About" }
        @Container {
          size: Size::new(144., 144.),
          @FittedBox {
            box_fit: BoxFit::Contain,
            @ { APP_ICON.clone() }
          }
        }
        @Text {
          text: "Powered by Ribir"
        }
      }
    }

  }
  .into_widget()
}

/// Configuration widget with sliders for focus, break durations, and cycles
fn setting_config() -> Widget<'static> {
  fn_widget! {
    let pomodoro = Provider::writer_of::<Pomodoro>(BuildCtx::get()).unwrap();
    let config = pomodoro.part_writer(PartialId::new("config".into()), |t| PartMut::new(&mut t.config));

    watch!($read(config);)
      .subscribe(move |_| $read(config).save());

    @Scrollbar {
      scrollable: Scrollable::Y,
      @Column {
        align_items: Align::Center,
        @Text { text: "Focus"}
        @Text {
          background: Color::GRAY.with_alpha(0.5),
          text: pipe!($read(config).focus).map(duration_to_mmss)
        }
        @slider! {
          class: FOCUS,
          min: 1.0,
          max: 90.0,
          value: $read(config).focus.as_secs_f32() / 60.0,
          divisions: Some(89),
          on_custom: move |e: &mut SliderChangedEvent| {
            let dur = Duration::from_secs_f32(e.data().to * 60.0);
            $write(config).focus = dur;
            if $read(pomodoro).state == PomodoroState::Focus {
              $write(pomodoro).current_remaining = dur;
              if $read(pomodoro).is_running() {
                $write(pomodoro).pause();
              }
            }
          }
        }
        @Text { text: "Short Break"}
        @Text {
          background: Color::GRAY.with_alpha(0.5),
          text: pipe!($read(config).short_break).map(duration_to_mmss)
        }
        @slider! {
          class: SHORT_BREAK,
          min: 1.0,
          max: 90.0,
          value: $read(config).short_break.as_secs_f32() / 60.0,
          divisions: Some(89),
          on_custom: move |e: &mut SliderChangedEvent| {
            let dur = Duration::from_secs_f32(e.data().to * 60.0);
            $write(config).short_break = dur;
            if $read(pomodoro).state == PomodoroState::ShortBreak {
              $write(pomodoro).current_remaining = dur;
              if $read(pomodoro).is_running() {
                $write(pomodoro).pause();
              }
            }
          }
        }
        @Text { text: "Long Break" }
        @Text {
          background: Color::GRAY.with_alpha(0.5),
          text: pipe!($read(config).long_break).map(duration_to_mmss)
        }
        @slider! {
          class: LONG_BREAK,
          min: 1.0,
          max: 90.0,
          value: $read(config).long_break.as_secs_f32() / 60.0,
          divisions: Some(89),
          on_custom: move |e: &mut SliderChangedEvent| {
            let dur = Duration::from_secs_f32(e.data().to * 60.0);
            $write(config).long_break = dur;
            if $read(pomodoro).state == PomodoroState::LongBreak {
              $write(pomodoro).current_remaining = dur;
              if $read(pomodoro).is_running() {
                $write(pomodoro).pause();
              }
            }
          }
        }
        @Text { text: "Cycles"}
        @Text {
          background: Color::GRAY.with_alpha(0.5),
          text: pipe!($read(config).cycles).map(|v| v.to_string())
        }
        @Slider {
          class: CYCLES,
          min: 1.0,
          max: 10.0,
          value: $read(config).cycles as f32,
          divisions: Some(9),
          on_custom: move |e: &mut SliderChangedEvent| {
            $write(config).cycles = e.data().to as u32;
          }
        }
        @Row {
          align_items: Align::Center,
          x: AnchorX::center(),
          margin: EdgeInsets::only_top(4.),
          @Checkbox {
            checked: pipe!($read(config).start_mini_mode),
            on_tap: move |_| {
              let current = $read(config).start_mini_mode;
              $write(config).start_mini_mode = !current;
            }
          }
          @Text {
            margin: EdgeInsets::only_left(8.),
            text: "Start in Mini Mode"
          }
        }
        @Row {
          align_items: Align::Center,
          x: AnchorX::center(),
          @Checkbox {
            checked: pipe!($read(config).auto_run),
            on_tap: move |_| {
              let current = $read(config).auto_run;
              $write(config).auto_run = !current;
            }
          }
          @Text {
            margin: EdgeInsets::only_left(8.),
            text: "Auto Run on Startup"
          }
        }
        @Row {
          align_items: Align::Center,
          x: AnchorX::center(),
          @Checkbox {
            checked: $read(config).always_on_top,
            on_custom: move |e: &mut CheckboxChanged| {
              $write(config).always_on_top = e.data().checked;
            }
          }
          @Text {
            margin: EdgeInsets::only_left(4.),
            text: "Keep Window on Top"
          }
        }
      }
    }
  }.into_widget()
}

/// Settings widget with tabbed interface for configuration and about pages
pub(crate) fn setting_page() -> Widget<'static> {
  fn_widget! {
    @Container {
      clamp: BoxClamp::EXPAND_X.with_fixed_height(CONTENT_HEIGHT),
      @Tabs{
        providers: [Provider::new(TabPos::Bottom)],
        x: AnchorX::left(),
        y: AnchorY::top(),
        @Tab {
          @{ "Settings" }
          @Icon { @ { svg_registry::get_or_default("settings") } }
          @fn_widget! {
            @FatObj {
              margin: EdgeInsets::new(10.0, 16.0, 10.0, 16.0),
              @ { setting_config() }
            }
          }
        }
        @Tab {
          @ { "about" }
          @Icon { @ { svg_registry::get_or_default("info") } }
          @fn_widget! { about() }
        }
      }
    }
  }
  .into_widget()
}

pub(crate) fn concise_page() -> Widget<'static> {
  fn_widget! {
    let pomodoro = Provider::writer_of::<Pomodoro>(BuildCtx::get()).unwrap();
    @Stack {
      @FittedBox {
        box_fit: BoxFit::Contain,
        @SpinnerProgress {
          class: CURRENT,
          value: pipe!(1. - $read(pomodoro).state_progress())
        }
      }

      @InParentLayout {
        @Column {
          x: AnchorX::center(),
          y: AnchorY::center(),
          align_items: Align::Center,
          @Text { text: "Pomodoro" }
          @H4 {
            text: @pipe!($read(pomodoro).current_remaining)
            .map(duration_to_mmss)
          }
          @FatObj {
            text_line_height: 24.,
            @{ run_button() }
          }
        }
      }
    }
  }
  .into_widget()
}
