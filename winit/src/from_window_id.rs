use std::any::Any;

use ribir_core::window::WindowId as RibirWindowId;
use winit::window::WindowId as WinitWindowId;

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct WrappedWindowId(WinitWindowId);

impl RibirWindowId for WrappedWindowId {
  fn as_any(&self) -> &dyn Any { self }
  fn eq(&self, other: &dyn RibirWindowId) -> bool { self.0 == WrappedWindowId::from(other).0 }
  fn box_clone(&self) -> Box<dyn RibirWindowId> { Box::new(self.clone()) }
}

impl From<WinitWindowId> for WrappedWindowId {
  fn from(value: WinitWindowId) -> Self { WrappedWindowId(value) }
}

impl From<WrappedWindowId> for WinitWindowId {
  fn from(val: WrappedWindowId) -> Self { val.0 }
}

impl From<Box<dyn RibirWindowId>> for WrappedWindowId {
  fn from(value: Box<dyn RibirWindowId>) -> Self {
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WinitWindowId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&Box<dyn RibirWindowId>> for WrappedWindowId {
  fn from(value: &Box<dyn RibirWindowId>) -> Self {
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WinitWindowId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&dyn RibirWindowId> for WrappedWindowId {
  fn from(value: &dyn RibirWindowId) -> Self {
    let x = value
      .as_any()
      .downcast_ref::<WinitWindowId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}
