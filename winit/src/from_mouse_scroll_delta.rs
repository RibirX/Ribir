use ribir_core::events::MouseScrollDelta as RibirMouseScrollDelta;
use winit::event::MouseScrollDelta as WinitMouseScrollDelta;

use crate::{from_event::ScaleToLogicalFactor, prelude::WrappedLogicalPosition};

pub struct WrappedMouseScrollDelta(WinitMouseScrollDelta, ScaleToLogicalFactor);

impl From<(WinitMouseScrollDelta, ScaleToLogicalFactor)> for WrappedMouseScrollDelta {
  fn from(value: (WinitMouseScrollDelta, ScaleToLogicalFactor)) -> Self {
    WrappedMouseScrollDelta(value.0, value.1)
  }
}

impl From<WrappedMouseScrollDelta> for RibirMouseScrollDelta {
  fn from(val: WrappedMouseScrollDelta) -> Self {
    match val.0 {
      WinitMouseScrollDelta::LineDelta(right, down) => {
        RibirMouseScrollDelta::LineDelta(right, down)
      }
      WinitMouseScrollDelta::PixelDelta(pos) => RibirMouseScrollDelta::PixelDelta(
        WrappedLogicalPosition::<f64>::from(pos.to_logical(val.1)).into(),
      ),
    }
  }
}
