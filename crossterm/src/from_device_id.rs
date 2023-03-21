use std::any::Any;

use ribir_core::events::PointerId as RibirPointerId;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CrosstermPointerId(usize);

impl CrosstermPointerId {
  pub fn zero() -> CrosstermPointerId { CrosstermPointerId(0) }
}

impl RibirPointerId for CrosstermPointerId {
  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
  fn equals(&self, other: &Box<dyn RibirPointerId>) -> bool {
    self.0
      == other
        .box_clone()
        .into_any()
        .downcast::<CrosstermPointerId>()
        .unwrap()
        .0
  }

  fn box_clone(&self) -> Box<dyn RibirPointerId> { Box::new(*self) }
}
