use std::fmt::Debug;

bitflags! {
    pub struct LayoutConstraints: u8 {
        const DECIDED_BY_SELF = 0;
        const EFFECTED_BY_PARENT = 1;
        const EFFECTED_BY_CHILDREN = 2;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Size {
  pub width: f64,
  pub height: f64,
}

#[derive(Debug, Clone)]
pub struct Position {
  pub x: f64,
  pub y: f64,
}
