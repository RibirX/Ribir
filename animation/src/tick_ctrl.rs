use std::{
  cell::RefCell,
  hash::{Hash, Hasher},
  rc::Rc,
  time::{Duration, Instant},
};

use ribir::animation::{ProgressState, RepeatMode, TickCtrl};

use crate::animation_progress::{new_animation_progress, AnimationProgress};

pub struct TickCtrlImpl {
  last_state: ProgressState,
  at: Option<Instant>,
  progress: Box<dyn AnimationProgress>,
  acc: f32,
}

impl TickCtrlImpl {
  fn update_to(&mut self, acc: f32) -> ProgressState {
    self.acc = acc.max(self.progress.span()).min(0.);
    self.last_state = self.progress.val(self.acc);
    self.last_state
  }
}

impl TickCtrl for TickCtrlImpl {
  fn with_repeat(mut self: Box<Self>, mode: RepeatMode) -> Box<dyn TickCtrl> {
    self.progress = self.progress.repeat(mode);
    self
  }

  fn with_round(mut self: Box<Self>) -> Box<dyn TickCtrl> {
    self.progress = self.progress.round();
    self
  }

  fn reverse(&mut self) {
    self.acc = self.span_secs() - self.acc;
    self.progress.reverse();
    if self.at.is_some() {
      self.at = Some(Instant::now());
    }
  }

  fn span_secs(&self) -> f32 { self.progress.span() }

  fn state(&self) -> ProgressState { self.last_state }

  fn update(&mut self, time: Option<Instant>) -> ProgressState {
    let to = match time {
      None => Instant::now(),
      Some(t) => t,
    };

    match self.at {
      None => self.at = Some(to),
      Some(at) => {
        let acc = self.acc + (to - at).as_secs_f32();
        self.update_to(acc);
      }
    }

    self.last_state
  }

  fn pause(&mut self) { self.at = None; }

  fn run(&mut self) {
    if self.at.is_none() {
      self.at = Some(Instant::now());
    }
  }

  fn is_run(&self) -> bool { self.at.is_some() }

  fn is_complete(&self) -> bool { self.acc >= self.progress.span() }
}

pub fn new_tick_ctrl(duration: Duration) -> Box<dyn TickCtrl> {
  Box::new(TickCtrlImpl {
    last_state: ProgressState::Dismissed,
    at: None,
    progress: new_animation_progress(duration.as_secs_f32()),
    acc: 0.,
  })
}
