#[derive(PartialEq, Copy, Clone)]
pub enum AnimationProgress {
  Dismissed,
  Between(f32),
  Finish,
}

impl AnimationProgress {
  pub fn value(&self) -> f32 {
    match self {
      AnimationProgress::Dismissed => 0.,
      AnimationProgress::Between(val) => *val,
      AnimationProgress::Finish => 1.,
    }
  }

  #[inline]
  pub fn is_dismissed(&self) -> bool { matches!(self, AnimationProgress::Dismissed) }

  #[inline]
  pub fn is_between(&self) -> bool { matches!(self, AnimationProgress::Between(_)) }

  #[inline]
  pub fn is_finish(&self) -> bool { matches!(self, AnimationProgress::Finish) }
}
