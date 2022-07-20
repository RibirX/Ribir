use std::time::Instant;

use super::{animation_progress::AnimationProgress, ProgressState};

pub struct AnimationState {
  last_state: Option<ProgressState>,
  at: Option<Instant>,
  progress: Box<dyn AnimationProgress>,
  acc: f32,
  delay: f32,
  reversed: bool,
}

impl AnimationState {
  pub fn new(progress: Box<dyn AnimationProgress>, delay: f32) -> AnimationState {
    AnimationState {
      last_state: None,
      at: None,
      progress,
      acc: 0.,
      delay,
      reversed: false,
    }
  }

  fn update_to(&mut self, acc: f32) -> Option<ProgressState> {
    self.acc = acc.min(self.progress.span() + self.delay).max(0.);
    if self.acc > self.delay {
      self.last_state = Some(self.progress.val(self.acc - self.delay));
    }
    self.last_state
  }

  pub fn update(&mut self, time: Option<Instant>) -> Option<ProgressState> {
    if !self.is_run() {
      return self.last_state;
    }

    let to = match time {
      None => Instant::now(),
      Some(t) => t,
    };

    match self.at {
      None => self.at = Some(to),
      Some(at) => {
        let acc = self.acc + (to - at).as_secs_f32();
        self.update_to(acc);
        self.at = Some(to);
      }
    }

    self.last_state
  }

  pub fn reverse(&mut self) {
    self.reversed = !self.reversed;
    if self.acc > self.delay {
      let acc = self.acc - self.delay;
      self.acc = self.span_secs() - acc + self.delay;
    }

    self.progress.reverse();
    if self.at.is_some() {
      self.at = Some(Instant::now());
    }
  }

  pub fn state(&self) -> Option<ProgressState> { self.last_state }

  pub fn stop(&mut self) { self.at = None; }

  pub fn start(&mut self) {
    if self.at.is_none() {
      self.at = Some(Instant::now());
    }
  }

  pub fn is_run(&self) -> bool { self.at.is_some() }

  pub fn is_complete(&self) -> bool { self.acc >= self.progress.span() }

  pub fn span_secs(&self) -> f32 { self.progress.span() }

  pub fn reset(&mut self) {
    self.acc = 0.;
    if self.reversed {
      self.progress.reverse();
    }
    self.last_state = None;
    self.reversed = false;
    self.at = None;
  }
}
