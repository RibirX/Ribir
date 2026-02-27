pub trait DeviceId: Send {
  fn as_any(&self) -> &dyn std::any::Any;
  fn is_same_device(&self, other: &dyn DeviceId) -> bool;
  fn clone_boxed(&self) -> Box<dyn DeviceId>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RibirDeviceId {
  #[default]
  Dummy,
  Custom(u64),
}

impl DeviceId for RibirDeviceId {
  fn as_any(&self) -> &dyn std::any::Any { self }

  fn is_same_device(&self, other: &dyn DeviceId) -> bool {
    other
      .as_any()
      .downcast_ref::<RibirDeviceId>()
      .is_some_and(|this| this == self)
  }

  fn clone_boxed(&self) -> Box<dyn DeviceId> { Box::new(*self) }
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
