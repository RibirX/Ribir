use std::any::Any;

use ribir_core::events::DeviceId as CDeviceId;
use winit::event::DeviceId as WDeviceId;

#[derive(PartialEq, Eq)]
pub struct RDeviceId(WDeviceId);

impl CDeviceId for RDeviceId {
  fn as_any(&self) -> &dyn Any { self }
  fn eq(&self, other: &dyn CDeviceId) -> bool { self.0 == RDeviceId::from(other).0 }
}

impl From<WDeviceId> for RDeviceId {
  fn from(value: WDeviceId) -> Self { RDeviceId(value) }
}

impl From<RDeviceId> for WDeviceId {
  fn from(val: RDeviceId) -> Self { val.0 }
}

impl From<Box<dyn CDeviceId>> for RDeviceId {
  fn from(value: Box<dyn CDeviceId>) -> Self {
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&Box<dyn CDeviceId>> for RDeviceId {
  fn from(value: &Box<dyn CDeviceId>) -> Self {
    let x = value
      .as_ref()
      .as_any()
      .downcast_ref::<WDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}

impl From<&dyn CDeviceId> for RDeviceId {
  fn from(value: &dyn CDeviceId) -> Self {
    let x = value
      .as_any()
      .downcast_ref::<WDeviceId>()
      .map(|v| v.to_owned())
      .unwrap();
    x.into()
  }
}
