use std::any::Any;

use ribir_core::events::PointerId as RibirPointerId;
use winit::event::DeviceId as WinitDeviceId;

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct WrappedPointerId(WinitDeviceId);

impl RibirPointerId for WrappedPointerId {
  fn as_any(&self) -> &dyn Any { self }
  fn eq(&self, other: &dyn RibirPointerId) -> bool { self.0 == WrappedPointerId::from(other).0 }
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
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WinitDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&Box<dyn RibirPointerId>> for WrappedPointerId {
  fn from(value: &Box<dyn RibirPointerId>) -> Self {
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WinitDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&dyn RibirPointerId> for WrappedPointerId {
  fn from(value: &dyn RibirPointerId) -> Self {
    let x = value
      .as_any()
      .downcast_ref::<WinitDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}
