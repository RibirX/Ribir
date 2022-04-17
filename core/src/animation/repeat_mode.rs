#[derive(PartialEq, Copy, Clone)]
pub enum RepeatMode {
  None,
  Repeat(u32),
  Infinity,
}

impl RepeatMode {
  pub fn val(&self) -> u32 {
    match self {
      RepeatMode::None => 1,
      RepeatMode::Repeat(val) => *val,
      RepeatMode::Infinity => u32::MAX,
    }
  }
}
