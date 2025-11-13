use ribir::prelude::*;

use crate::pomodoro::{Pomodoro, PomodoroState};

class_names! {
  CURRENT,
  FOCUS,
  SHORT_BREAK,
  LONG_BREAK,
  CYCLES,
  WINDOW_BAR,
  MINI_WINDOW_BAR,
}

const FOCUS_COLOR: Color = Color::from_rgb(200, 50, 50);
const SHORT_BREAK_COLOR: Color = Color::from_rgb(50, 140, 50);
const LONG_BREAK_COLOR: Color = Color::from_rgb(50, 50, 140);
const CYCLES_COLOR: Color = Color::from_rgb(180, 180, 50);

pub const FULL_WIDTH: f32 = 360.0;
pub const FULL_HEIGHT: f32 = 486.0;

pub const MINI_WIDTH: f32 = 180.0;
pub const MINI_HEIGHT: f32 = 206.0;

pub const WINDOW_BAR_HEIGHT: f32 = 30.0;
pub const MINI_WINDOW_BAR_HEIGHT: f32 = 24.0;

pub const CONTENT_HEIGHT: f32 = FULL_HEIGHT - WINDOW_BAR_HEIGHT;
// pub const MINI_CONTENT_HEIGHT: f32 = MINI_HEIGHT - MINI_WINDOW_BAR_HEIGHT;

fn change_color(w: Widget<'_>) -> Widget<'_> {
  rdl! {
    let base_color = Stateful::new(FOCUS_COLOR);
    let pomodoro = Provider::watcher_of::<Pomodoro>(BuildCtx::get()).unwrap();
    watch!($read(pomodoro).state)
      .subscribe(move |state| {
        let color = match state {
          PomodoroState::Focus => FOCUS_COLOR,
          PomodoroState::ShortBreak => SHORT_BREAK_COLOR,
          PomodoroState::LongBreak => LONG_BREAK_COLOR,
        };
        *$write(base_color) = color;
      });
    let mut w = FatObj::new(w);

    @(w) {
      providers: [Provider::watcher(base_color.clone_writer())],
    }
  }
  .into_widget()
}

pub fn styles() -> Vec<Provider> {
  vec![
    Class::provider(CURRENT, change_color),
    Class::provider(
      FOCUS,
      style_class! {
        providers: [Provider::new(FOCUS_COLOR)]
      },
    ),
    Class::provider(
      SHORT_BREAK,
      style_class! {
        providers: [Provider::new(SHORT_BREAK_COLOR)]
      },
    ),
    Class::provider(
      LONG_BREAK,
      style_class! {
        providers: [Provider::new(LONG_BREAK_COLOR)]
      },
    ),
    Class::provider(
      CYCLES,
      style_class! {
        providers: [Provider::new(CYCLES_COLOR)]
      },
    ),
    Class::provider(
      WINDOW_BAR,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(f32::INFINITY, WINDOW_BAR_HEIGHT)),
        background: Palette::of(BuildCtx::get()).surface_container_low(),
        text_line_height: 24.,
      },
    ),
    Class::provider(
      MINI_WINDOW_BAR,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(f32::INFINITY, MINI_WINDOW_BAR_HEIGHT)),
        background: Palette::of(BuildCtx::get()).surface_container_low().apply_alpha(0.8),
        text_line_height: 16.,
      },
    ),
  ]
}
