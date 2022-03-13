#[derive(PartialEq, Copy, Clone)]
pub enum ProgressState {
  Dismissed,
  Between(f32),
  Finish,
}

impl ProgressState {
  pub fn val(&self) -> f32 {
    match self {
      ProgressState::Dismissed => 0.,
      ProgressState::Between(val) => *val,
      ProgressState::Finish => 1.,
    }
  }

  pub fn is_dismissed(&self) -> bool {
    match self {
      ProgressState::Dismissed => true,
      _ => false,
    }
  }

  pub fn is_between(&self) -> bool {
    match self {
      ProgressState::Between(_) => true,
      _ => false,
    }
  }

  pub fn is_finish(&self) -> bool {
    match self {
      ProgressState::Finish => true,
      _ => false,
    }
  }
}
