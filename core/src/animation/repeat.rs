#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Repeat {
  None,
  Repeat(u32),
  Infinite,
}

impl Repeat {
  pub fn repeat_cnt(&self) -> u32 {
    match self {
      Repeat::None => 1,
      Repeat::Repeat(val) => *val,
      Repeat::Infinite => u32::MAX,
    }
  }

  pub fn is_infinite(&self) -> bool { matches!(self, Repeat::Infinite) }

  pub fn is_none(&self) -> bool { matches!(self, Repeat::None) }

  pub fn is_repeat(&self) -> bool { matches!(self, Repeat::Repeat(_)) }
}

impl Default for Repeat {
  fn default() -> Self { Repeat::None }
}
