use std::time::Duration;

use ribir::prelude::*;
use rodio::Sink;

use crate::{audio, config::PomodoroConfig};

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum PomodoroState {
  #[default]
  Focus,
  ShortBreak,
  LongBreak,
}

pub const UPDATE_INTERVAL: Duration = Duration::from_millis(250);

#[declare]
pub struct Pomodoro {
  #[declare(default = PomodoroConfig::load())]
  pub config: PomodoroConfig,
  #[declare(skip)]
  pub state: PomodoroState,
  #[declare(skip)]
  pub running_guard: Option<SubscriptionGuard<BoxedSubscription>>,
  #[declare(skip)]
  pub rounds: u32,

  #[declare(skip, default = Duration::ZERO)]
  pub current_remaining: Duration,

  #[declare(skip)]
  #[cfg(not(target_arch = "wasm32"))]
  pub audio_sink: Option<Sink>,

  #[declare(skip, default = 1.0)]
  pub volume: f32,
}

// Timer controller implementation
impl Pomodoro {
  // Reset the timer
  pub fn reset(&mut self) {
    self.running_guard.take();
    self.state = PomodoroState::Focus;
    self.current_remaining = self.config.focus;
    self.rounds = 0;
  }

  // Transition to the next state
  pub fn next_state(&mut self) {
    match self.state {
      PomodoroState::Focus => {
        self.rounds = (self.rounds + 1) % self.config.cycles;
        if self.rounds == 0 {
          self.state = PomodoroState::LongBreak;
          self.current_remaining = self.config.long_break;
        } else {
          self.state = PomodoroState::ShortBreak;
          self.current_remaining = self.config.short_break;
        }
      }
      PomodoroState::ShortBreak => {
        self.state = PomodoroState::Focus;
        self.current_remaining = self.config.focus;
      }
      PomodoroState::LongBreak => {
        self.state = PomodoroState::Focus;
        self.current_remaining = self.config.focus;
      }
    }
  }

  pub fn is_running(&self) -> bool { self.running_guard.is_some() }

  pub fn elapse(&mut self, mut delta: Duration) -> bool {
    if !self.is_running() {
      return false;
    }
    let mut state_changed = false;
    loop {
      if delta == Duration::ZERO {
        break;
      }
      if self.current_remaining > delta {
        self.current_remaining -= delta;
        break;
      } else {
        delta -= self.current_remaining;
        state_changed = true;
        self.next_state();
      }
    }
    state_changed
  }

  pub fn state_duration(&self) -> Duration {
    match self.state {
      PomodoroState::Focus => self.config.focus,
      PomodoroState::ShortBreak => self.config.short_break,
      PomodoroState::LongBreak => self.config.long_break,
    }
  }

  pub fn state_progress(&self) -> f32 {
    let total = self.state_duration().as_secs_f32();
    let remaining = self.current_remaining.as_secs_f32();
    1. - (remaining / total)
  }

  pub fn run(this: &impl StateWriter<Value = Self>, update_interval: Duration) {
    let mut start = Instant::now();
    let boxed: LocalBoxedObservable<'_, usize, _> = Local::interval(update_interval).box_it();

    let this = this.clone_writer();
    let this2 = this.clone_writer();
    let guard: SubscriptionGuard<BoxedSubscription> = boxed
      .subscribe(move |_| {
        let state_changed = this.write().elapse(start.elapsed());
        if state_changed {
          let volume = this.read().volume;
          audio::play_notification_sound(volume);
        }
        start = Instant::now();
      })
      .unsubscribe_when_dropped();
    this2.write().running_guard = Some(guard);
  }

  pub fn pause(&mut self) { self.running_guard.take(); }
}
