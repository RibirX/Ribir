use crate::render::render_ctx::*;
use indextree::*;
use std::fmt::Debug;

use crate::render::render_ctx::RenderCtx;

bitflags! {
    pub struct LayoutConstraints: u8 {
        const DECIDED_BY_SELF = 0;
        const EFFECTED_BY_PARENT = 1;
        const EFFECTED_BY_CHILDREN = 2;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Size {
  pub width: i32,
  pub height: i32,
}

#[derive(Debug, Clone)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}
