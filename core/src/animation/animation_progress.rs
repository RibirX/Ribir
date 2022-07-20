use super::{ProgressState, RepeatMode};

struct ProgressWithRepeat(Box<dyn AnimationProgress>, RepeatMode);

struct ProgressWithReverse(Box<dyn AnimationProgress>);

pub trait AnimationProgress {
  fn val(&self, v: f32) -> ProgressState;
  fn span(&self) -> f32;

  fn reverse(&self) -> Box<dyn AnimationProgress>;
  fn round(&self) -> Box<dyn AnimationProgress>;
  #[inline]
  fn repeat(&self, mode: RepeatMode) -> Box<dyn AnimationProgress> {
    if mode.val() == 1 {
      self.clone_box()
    } else {
      Box::new(ProgressWithRepeat(self.clone_box(), mode))
    }
  }

  fn clone_box(&self) -> Box<dyn AnimationProgress>;
}

pub(crate) fn new_animation_progress(span: f32) -> Box<dyn AnimationProgress> {
  Box::new(Span { span })
}

#[derive(Copy, Clone)]
struct Span {
  span: f32,
}
impl AnimationProgress for Span {
  fn val(&self, v: f32) -> ProgressState {
    if v >= self.span {
      ProgressState::Finish
    } else if v <= 0. {
      ProgressState::Dismissed
    } else {
      ProgressState::Between(v / self.span)
    }
  }

  fn span(&self) -> f32 { self.span }

  fn reverse(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithReverse(self.clone_box()))
  }

  fn round(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithRound(self.clone_box(), self.reverse()))
  }

  fn clone_box(&self) -> Box<dyn AnimationProgress> { Box::new(*self) }
}

impl AnimationProgress for ProgressWithReverse {
  fn val(&self, v: f32) -> ProgressState {
    match self.0.val(v) {
      ProgressState::Between(v) => ProgressState::Between(1. - v),
      ProgressState::Dismissed => ProgressState::Finish,
      ProgressState::Finish => ProgressState::Dismissed,
    }
  }

  fn span(&self) -> f32 { self.0.span() }

  fn reverse(&self) -> Box<dyn AnimationProgress> { self.0.clone_box() }
  fn round(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithRound(self.clone_box(), self.reverse()))
  }

  fn clone_box(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithReverse(self.0.clone_box()))
  }
}

impl AnimationProgress for ProgressWithRepeat {
  fn val(&self, v: f32) -> ProgressState {
    let round = v / self.0.span();
    if round <= 0. {
      self.0.val(0.)
    } else if 0. < round && round < self.1.val() as f32 {
      let val = self.0.val(v % self.0.span()).val();
      ProgressState::Between(val)
    } else {
      self.0.val(self.0.span())
    }
  }

  fn span(&self) -> f32 {
    match self.1 {
      RepeatMode::Infinity => f32::MAX,
      _ => self.1.val() as f32 * self.0.span(),
    }
  }

  fn reverse(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithReverse(self.clone_box()))
  }
  fn round(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithRound(self.clone_box(), self.reverse()))
  }

  fn clone_box(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithRepeat(self.0.clone_box(), self.1))
  }
}

struct ProgressWithRound(Box<dyn AnimationProgress>, Box<dyn AnimationProgress>);

impl AnimationProgress for ProgressWithRound {
  fn val(&self, v: f32) -> ProgressState {
    let time = v / self.0.span();
    if time >= 2. {
      ProgressState::Finish
    } else if time <= 0. {
      ProgressState::Dismissed
    } else if time <= 1. {
      ProgressState::Between(self.0.val(v).val())
    } else {
      ProgressState::Between(self.0.val(self.span() - v).val())
    }
  }

  fn span(&self) -> f32 { self.0.span() * 2. }

  fn reverse(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithReverse(self.clone_box()))
  }

  fn round(&self) -> Box<dyn AnimationProgress> { self.clone_box() }

  fn clone_box(&self) -> Box<dyn AnimationProgress> {
    Box::new(ProgressWithRound(self.0.clone_box(), self.1.clone_box()))
  }
}

#[cfg(test)]
mod tests {
  use super::{new_animation_progress, ProgressState, RepeatMode};

  #[test]
  fn test_progress() {
    let calc = new_animation_progress(5.);
    assert!(calc.val(0.) == ProgressState::Dismissed);
    assert!(calc.val(2.5) == ProgressState::Between(0.5));
    assert!(calc.val(5.) == ProgressState::Finish);
    assert!(calc.val(5.1) == ProgressState::Finish);
  }

  #[test]
  fn test_repeat() {
    let calc = new_animation_progress(5.).repeat(RepeatMode::Repeat(3));
    assert!(calc.val(0.) == ProgressState::Dismissed);
    assert!(calc.val(2.5) == ProgressState::Between(0.5));
    assert!(calc.val(5.) == ProgressState::Between(0.));
    assert!(calc.val(12.) == ProgressState::Between(0.4));
  }

  #[test]
  fn test_round() {
    let calc = new_animation_progress(5.)
      .round()
      .repeat(RepeatMode::Repeat(3));
    assert!(calc.val(0.) == ProgressState::Dismissed);
    assert!(calc.val(2.5) == ProgressState::Between(0.5));
    assert!(calc.val(5.) == ProgressState::Between(1.));
    assert!(calc.val(9.) == ProgressState::Between(0.2));
  }

  #[test]
  fn test_reverse() {
    let calc = new_animation_progress(5.)
      .round()
      .repeat(RepeatMode::Repeat(3))
      .reverse();
    assert!(calc.val(0.) == ProgressState::Finish);
    assert!(calc.val(2.5) == ProgressState::Between(0.5));
    assert!(calc.val(5.) == ProgressState::Between(0.));
    assert!(calc.val(9.) == ProgressState::Between(0.8));
  }
}
