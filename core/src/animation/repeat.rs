#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Repeat {
  Repeat(u32),
  Infinite,
}

impl Repeat {
  pub fn repeat_cnt(&self) -> u32 {
    match self {
      Repeat::Repeat(val) => *val,
      Repeat::Infinite => u32::MAX,
    }
  }

  pub fn is_infinite(&self) -> bool { matches!(self, Repeat::Infinite) }

  pub fn is_repeat(&self) -> bool { matches!(self, Repeat::Repeat(_)) }
}
