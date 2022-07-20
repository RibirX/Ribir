use crate::prelude::*;

use crate::prelude::BuildCtx;
use crate::ticker::Ticker;

use super::RepeatMode;
use crate::animation::{new_animation_progress, AnimationState};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare)]
pub struct Transition {
  pub delay: Option<Duration>,
  pub duration: Duration,
  pub repeat: RepeatMode,
  pub curve: Option<CurveGenerator>,
}

pub trait AnimationTransition {
  type Observable: AnimationObservable;
  fn animation_ctrl(&self, ticker: Ticker) -> Self::Observable;
}

impl AnimationTransition for Transition {
  type Observable = AnimationController;
  fn animation_ctrl(&self, ticker: Ticker) -> Self::Observable {
    let progress = new_animation_progress(self.duration.as_secs_f32()).repeat(self.repeat);
    let delay = match self.delay {
      None => 0.,
      Some(t) => t.as_secs_f32(),
    };
    AnimationController::new(
      ticker,
      Rc::new(RefCell::new(AnimationState::new(progress, delay))),
      self.curve.as_ref().map_or(linear(), |g| (*g)()),
    )
  }
}
