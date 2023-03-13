use ribir_core::events::MouseScrollDelta as RibirMouseScrollDelta;
use winit::event::MouseScrollDelta as WinitMouseScrollDelta;

use crate::prelude::WrappedPhysicalPosition;

pub struct WrappedMouseScrollDelta(WinitMouseScrollDelta);

impl From<WinitMouseScrollDelta> for WrappedMouseScrollDelta {
  fn from(value: WinitMouseScrollDelta) -> Self { WrappedMouseScrollDelta(value) }
}

impl From<WrappedMouseScrollDelta> for WinitMouseScrollDelta {
  fn from(val: WrappedMouseScrollDelta) -> Self { val.0 }
}

impl From<WrappedMouseScrollDelta> for RibirMouseScrollDelta {
  fn from(val: WrappedMouseScrollDelta) -> Self {
    match val.0 {
      WinitMouseScrollDelta::LineDelta(right, down) => {
        RibirMouseScrollDelta::LineDelta(right, down)
      }
      WinitMouseScrollDelta::PixelDelta(pos) => {
        RibirMouseScrollDelta::PixelDelta(WrappedPhysicalPosition::from(pos).into())
      }
    }
  }
}

impl From<RibirMouseScrollDelta> for WrappedMouseScrollDelta {
  fn from(value: RibirMouseScrollDelta) -> WrappedMouseScrollDelta {
    let es = match value {
      RibirMouseScrollDelta::LineDelta(right, down) => {
        WinitMouseScrollDelta::LineDelta(right, down)
      }
      RibirMouseScrollDelta::PixelDelta(pos) => {
        WinitMouseScrollDelta::PixelDelta(WrappedPhysicalPosition::from(pos).into())
      }
    };
    es.into()
  }
}
