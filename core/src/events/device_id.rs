pub trait DeviceId {
  fn as_any(&self) -> &dyn std::any::Any;
  fn is_same_device(&self, other: &dyn DeviceId) -> bool;
  fn clone_boxed(&self) -> Box<dyn DeviceId>;
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct DummyDeviceId;

impl DeviceId for DummyDeviceId {
  fn as_any(&self) -> &dyn std::any::Any { self }
  fn is_same_device(&self, other: &dyn DeviceId) -> bool {
    other
      .as_any()
      .downcast_ref::<DummyDeviceId>()
      .is_some_and(|this| this == self)
  }

  fn clone_boxed(&self) -> Box<dyn DeviceId> { Box::new(*self) }
}

impl Clone for Box<dyn DeviceId> {
  fn clone(&self) -> Box<dyn DeviceId> { self.clone_boxed() }
}
