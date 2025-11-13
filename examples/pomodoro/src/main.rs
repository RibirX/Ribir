#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod pomodoro;
mod ui;

use pomodoro::Pomodoro;
use ribir::{material, prelude::*};

use crate::ui::{
  APP_ICON, load_icons,
  styles::{MINI_HEIGHT, MINI_WIDTH},
};

pub fn main() {
  load_icons();
  App::run(fn_widget! { @Pomodoro {}})
    .with_app_theme(material::purple::light)
    .with_size(Size::new(MINI_WIDTH, MINI_HEIGHT))
    .with_resizable(false)
    .with_icon(&APP_ICON)
    .with_decorations(false)
    .with_title("Pomodoro Timer");
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;
  use crate::config::PomodoroConfig;

  fn mini_pomodoro() -> Widget<'static> {
    load_icons();
    let mut config = PomodoroConfig::default_config();
    config.auto_run = false;
    config.start_mini_mode = true;
    fn_widget! {
      @Pomodoro {
        config: config,
      }
    }
    .into_widget()
  }

  fn full_pomodoro() -> Widget<'static> {
    load_icons();
    let mut config = PomodoroConfig::default_config();
    config.auto_run = false;
    config.start_mini_mode = false;
    fn_widget! {
      @Pomodoro {
        config: config,
      }
    }
    .into_widget()
  }

  widget_image_tests!(
    mini_pomodoro,
    WidgetTester::new(mini_pomodoro)
      .with_wnd_size(Size::new(320., 240.))
      .with_comparison(0.0005)
  );
  widget_image_tests!(
    full_pomodoro,
    WidgetTester::new(full_pomodoro)
      .with_wnd_size(Size::new(320., 240.))
      .with_comparison(0.0005)
  );
}
