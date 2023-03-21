use std::any::Any;

use ribir_core::window::WindowId as RibirWindowId;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CrosstermWindowId(usize);

impl CrosstermWindowId {
  pub fn zero() -> CrosstermWindowId { CrosstermWindowId(0) }
}

impl RibirWindowId for CrosstermWindowId {
  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
  fn equals(&self, other: &Box<dyn RibirWindowId>) -> bool {
    self.0
      == other
        .box_clone()
        .into_any()
        .downcast::<CrosstermWindowId>()
        .unwrap()
        .0
  }

  fn box_clone(&self) -> Box<dyn RibirWindowId> { Box::new(*self) }
}

impl From<Box<dyn RibirWindowId>> for CrosstermWindowId {
  fn from(value: Box<dyn RibirWindowId>) -> Self {
    *value.into_any().downcast::<CrosstermWindowId>().unwrap()
  }
}
