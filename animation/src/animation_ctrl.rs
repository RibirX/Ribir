use ribir::{
  animation::{AnimationCtrl, ProgressState, TickerAnimationCtrl, TickerRunningHandle},
  prelude::Observable,
  widget::LocalSubject,
  widget::Observer,
};
use rxrust::ops::box_it::LocalBoxOp;

use crate::curve::Curve;

struct AnimationCtrlImpl {
  last_state: ProgressState,
  subject: LocalSubject<'static, f32, ()>,
  curve: Option<Box<dyn Curve>>,
}

impl AnimationCtrl for AnimationCtrlImpl {
  fn state(&self) -> ProgressState { self.last_state }

  fn value(&self) -> f32 {
    return match &self.curve {
      Some(c) => c.transform(self.last_state.val()),
      None => self.last_state.val(),
    };
  }

  fn subject(&mut self) -> LocalBoxOp<'static, f32, ()> { self.subject.clone().box_it() }

  fn step(&mut self, step: f32) {
    let state = match step + self.last_state.val() {
      val if val <= 0. => ProgressState::Dismissed,
      val if val >= 1. => ProgressState::Finish,
      val => ProgressState::Between(val),
    };

    self.update_to(state);
  }

  fn update_to(&mut self, state: ProgressState) {
    self.last_state = state;
    self.subject.next(self.value());
  }
}

pub trait AnimationByTicker {
  fn trigger_by(
    self: Box<Self>,
    ticker: &mut dyn TickerAnimationCtrl,
  ) -> Box<dyn TickerRunningHandle>;
}

impl AnimationByTicker for dyn AnimationCtrl {
  fn trigger_by(
    mut self: Box<Self>,
    ticker: &mut dyn TickerAnimationCtrl,
  ) -> Box<dyn TickerRunningHandle> {
    ticker.listen(Box::new(move |p| self.update_to(p)))
  }
}

pub fn new_animation_ctrl(curve: Option<Box<dyn Curve>>) -> Box<dyn AnimationCtrl> {
  Box::new(AnimationCtrlImpl {
    last_state: ProgressState::Dismissed,
    subject: <_>::default(),
    curve,
  })
}

#[cfg(test)]
mod tests {
  use ribir::prelude::SubscribeNext;

  use crate::animation_ctrl::*;
  #[test]
  fn test_animation_ctrl() {
    let mut ctrl = new_animation_ctrl(None);
    assert!(ctrl.state() == ProgressState::Dismissed);
    ctrl.step(0.5);
    assert!(ctrl.state() == ProgressState::Between(0.5));
    ctrl.step(0.6);
    assert!(ctrl.state() == ProgressState::Finish);
  }

  #[test]
  fn test_animation_ctrl_subject() {
    let mut ctrl = new_animation_ctrl(None);
    let mut progress: f32 = 0.;
    let ptr = &mut progress as *mut f32;
    ctrl.subject().subscribe(move |v| unsafe { *ptr = v });

    ctrl.step(0.2);
    assert!(progress == 0.2);
    ctrl.step(-0.1);
    assert!(progress == 0.1);
    ctrl.step(2.);
    assert!(progress == 1.);
  }
}
