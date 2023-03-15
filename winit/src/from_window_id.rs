use std::any::Any;

use ribir_core::window::WindowId as RibirWindowId;
use winit::window::WindowId as WinitWindowId;

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct WrappedWindowId(WinitWindowId);

impl RibirWindowId for WrappedWindowId {
  // fn as_any(&self) -> &dyn Any { self }
  fn into_any(self: Box<Self>) -> Box<dyn Any> { self }

  fn equals(&self, other: &Box<dyn RibirWindowId>) -> bool {
    self.0
      == other
        .box_clone()
        .into_any()
        .downcast::<WrappedWindowId>()
        .unwrap()
        .0
  }
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
    *value.into_any().downcast::<WrappedWindowId>().unwrap()
  }
}

impl From<WrappedWindowId> for Box<dyn RibirWindowId> {
  fn from(value: WrappedWindowId) -> Self { Box::new(value) }
}

// impl From<&Box<dyn RibirWindowId>> for WrappedWindowId {
//     fn from(value: &Box<dyn RibirWindowId>) -> Self {
//         // let x = *value;
//     value.as_boxed_any().downcast_ref::<WrappedWindowId>().unwrap()
//   }
// }

// impl From<Box<dyn RibirWindowId>> for WrappedWindowId {
//   fn from(value: Box<dyn RibirWindowId>) -> Self {
//     value
//       .into_any()
//       .downcast::<WrappedWindowId>()
//       .unwrap()
//   }
// }

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ribir_window_id_into_wrapped() {
    let d1: WinitWindowId = 3_u64.into();
    let d2: WinitWindowId = 3_u64.into();
    assert_eq!(d1, d2);
    let winit_window_id = unsafe { WinitWindowId::dummy() };
    let wrapped = WrappedWindowId::from(winit_window_id);
    let boxed_ribir_window_id: Box<dyn RibirWindowId> = Box::new(wrapped.clone());
    let wrapped2: WrappedWindowId = boxed_ribir_window_id.into();
    assert!(wrapped2.equals(&wrapped.into()));
  }
}
