use std::any::Any;

use ribir_core::events::PointerId as RibirPointerId;
use winit::event::DeviceId as WinitDeviceId;

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct WrappedPointerId(WinitDeviceId);

impl RibirPointerId for WrappedPointerId {
  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }
  fn equals(&self, other: &Box<dyn RibirPointerId>) -> bool {
    self.0 == WrappedPointerId::from(other).0
  }
  fn box_clone(&self) -> Box<dyn RibirPointerId> { Box::new(self.clone()) }
}

impl From<WinitDeviceId> for WrappedPointerId {
  fn from(value: WinitDeviceId) -> Self { WrappedPointerId(value) }
}

impl From<WrappedPointerId> for WinitDeviceId {
  fn from(val: WrappedPointerId) -> Self { val.0 }
}

impl From<Box<dyn RibirPointerId>> for WrappedPointerId {
  fn from(value: Box<dyn RibirPointerId>) -> Self {
    *value
      .box_clone()
      .into_any()
      .downcast::<WrappedPointerId>()
      .unwrap()
  }
}

impl From<&Box<dyn RibirPointerId>> for WrappedPointerId {
  fn from(value: &Box<dyn RibirPointerId>) -> Self {
    *(value
      .box_clone()
      .into_any()
      .downcast::<WrappedPointerId>()
      .unwrap())
  }
}
