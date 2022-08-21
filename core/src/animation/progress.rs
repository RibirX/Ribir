#[derive(PartialEq, Copy, Clone)]
pub enum AnimateProgress {
  Dismissed,
  Between(f32),
  Finish,
}

impl AnimateProgress {
  pub fn value(&self) -> f32 {
    match self {
      AnimateProgress::Dismissed => 0.,
      AnimateProgress::Between(val) => *val,
      AnimateProgress::Finish => 1.,
    }
  }

  #[inline]
  pub fn is_dismissed(&self) -> bool { matches!(self, AnimateProgress::Dismissed) }

  #[inline]
  pub fn is_between(&self) -> bool { matches!(self, AnimateProgress::Between(_)) }

  #[inline]
  pub fn is_finish(&self) -> bool { matches!(self, AnimateProgress::Finish) }
}
