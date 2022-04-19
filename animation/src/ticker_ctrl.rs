use std::{
  cell::RefCell,
  rc::Rc,
  time::{Duration, Instant},
};

use ribir::animation::{ProgressState, RepeatMode, TickerRunningHandle};

use crate::animation_progress::{new_animation_progress, AnimationProgress};

pub(crate) struct RawTickerCtrl {
  last_state: ProgressState,
  at: Option<Instant>,
  progress: Box<dyn AnimationProgress>,
  acc: f32,
  call_backs: Vec<ListenHandle>,
  reversed: bool,
}

impl RawTickerCtrl {
  pub fn new(duration: Duration) -> RawTickerCtrl {
    RawTickerCtrl {
      last_state: ProgressState::Dismissed,
      at: None,
      progress: new_animation_progress(duration.as_secs_f32()),
      acc: 0.,
      call_backs: Vec::default(),
      reversed: false,
    }
  }

  fn update_to(&mut self, acc: f32) -> ProgressState {
    self.acc = acc.min(self.progress.span()).max(0.);
    self.last_state = self.progress.val(self.acc);

    self
      .call_backs
      .drain_filter(|h| {
        if h.is_disposed() {
          true
        } else {
          h.call(self.last_state);
          false
        }
      })
      .for_each(drop);
    self.last_state
  }

  pub fn update(&mut self, time: Option<Instant>) -> ProgressState {
    if !self.is_run() || self.is_complete() {
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
    self.acc = self.span_secs() - self.acc;
    self.progress.reverse();
    if self.at.is_some() {
      self.at = Some(Instant::now());
    }
  }

  pub fn state(&self) -> ProgressState { self.last_state }

  pub fn pause(&mut self) { self.at = None; }

  pub fn start(&mut self) {
    if self.at.is_none() {
      self.at = Some(Instant::now());
    }
  }

  pub fn is_run(&self) -> bool { self.at.is_some() }

  pub fn is_complete(&self) -> bool { self.acc >= self.progress.span() }

  pub fn with_repeat(&mut self, mode: RepeatMode) { self.progress = self.progress.repeat(mode); }

  pub fn with_round(&mut self) { self.progress = self.progress.round(); }

  pub fn span_secs(&self) -> f32 { self.progress.span() }

  pub fn listen(&mut self, cb: Box<dyn FnMut(ProgressState)>) -> Box<dyn TickerRunningHandle> {
    let wrap = Rc::new(RefCell::new(Some(cb)));
    self.call_backs.push(ListenHandle { cb: wrap.clone() });
    Box::new(ListenHandle { cb: wrap.clone() })
  }

  pub fn restart(&mut self, run: bool) {
    self.acc = 0.;
    if self.reversed {
      self.progress.reverse();
    }
    self.last_state = self.progress.val(0.);
    self.reversed = false;
    self.at = if run { Some(Instant::now()) } else { None };
  }

  pub fn force_done(&mut self) {
    if self.is_complete() {
      return;
    }
    self.update_to(self.span_secs());
  }
}

struct ListenHandle {
  cb: Rc<RefCell<Option<Box<dyn FnMut(ProgressState)>>>>,
}

impl ListenHandle {
  fn call(&self, state: ProgressState) {
    if let Some(cb) = self.cb.borrow_mut().as_mut() {
      (cb)(state);
    }
  }

  fn is_disposed(&self) -> bool { self.cb.borrow().is_none() }
}

impl TickerRunningHandle for ListenHandle {
  fn dispose(&mut self) { *self.cb.borrow_mut() = None; }
}

#[cfg(test)]
mod tests {
  use super::*;

  use ribir::animation::RepeatMode;
  use std::time::Duration;

  fn progress_eq(p1: ProgressState, p2: ProgressState) -> bool {
    if p1 == p2 {
      return true;
    }
    if f32::abs(p1.val() - p2.val()) > 0.1 {
      return false;
    }

    if (p1.is_between() && p2.is_between())
      || (p1.is_finish() && p2.is_finish())
      || (p1.is_dismissed() && p2.is_dismissed())
    {
      return true;
    } else {
      return false;
    }
  }

  #[test]
  fn test_progress() {
    let mut ctrl = RawTickerCtrl::new(Duration::from_millis(500));
    ctrl.start();
    let mut to = Instant::now() + Duration::from_millis(100);
    ctrl.update(Some(to));
    assert!(progress_eq(ctrl.state(), ProgressState::Between(0.2)));
    to += Duration::from_millis(450);
    ctrl.update(Some(to));
    assert!(progress_eq(ctrl.state(), ProgressState::Finish));
  }

  #[test]
  fn test_repeat() {
    let mut ctrl = RawTickerCtrl::new(Duration::from_millis(500));
    ctrl.with_repeat(RepeatMode::Repeat(5));
    ctrl.start();

    let mut to = Instant::now() + Duration::from_millis(600);
    ctrl.update(Some(to));
    assert!(progress_eq(ctrl.state(), ProgressState::Between(0.2)));
    to += Duration::from_millis(2000);
    ctrl.update(Some(to));
    assert!(progress_eq(ctrl.state(), ProgressState::Finish));
  }

  #[test]
  fn test_stop() {
    let mut ctrl = RawTickerCtrl::new(Duration::from_millis(500));
    ctrl.start();

    let mut now = Instant::now();
    ctrl.update(Some(now + Duration::from_millis(100)));
    ctrl.pause();
    assert!(progress_eq(ctrl.state(), ProgressState::Between(0.2)));

    ctrl.update(Some(now + Duration::from_millis(2000)));
    assert!(progress_eq(ctrl.state(), ProgressState::Between(0.2)));

    ctrl.start();
    now = Instant::now();

    now += Duration::from_millis(100);
    ctrl.update(Some(now));
    assert!(progress_eq(ctrl.state(), ProgressState::Between(0.4)));

    now += Duration::from_millis(300);
    ctrl.update(Some(now));
    assert!(ctrl.state().is_finish());
  }
}
