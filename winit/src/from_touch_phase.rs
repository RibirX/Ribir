use ribir_core::events::TouchPhase as RibirTouchPhase;
use winit::event::TouchPhase as WinitTouchPhase;

pub struct WrappedTouchPhase(WinitTouchPhase);

impl From<WinitTouchPhase> for WrappedTouchPhase {
  fn from(value: WinitTouchPhase) -> Self { WrappedTouchPhase(value) }
}

impl From<WrappedTouchPhase> for WinitTouchPhase {
  fn from(val: WrappedTouchPhase) -> Self { val.0 }
}

impl From<WrappedTouchPhase> for RibirTouchPhase {
  fn from(val: WrappedTouchPhase) -> Self {
    match val.0 {
      WinitTouchPhase::Started => RibirTouchPhase::Started,
      WinitTouchPhase::Moved => RibirTouchPhase::Moved,
      WinitTouchPhase::Ended => RibirTouchPhase::Ended,
      WinitTouchPhase::Cancelled => RibirTouchPhase::Cancelled,
    }
  }
}

impl From<RibirTouchPhase> for WrappedTouchPhase {
  fn from(value: RibirTouchPhase) -> WrappedTouchPhase {
    let tb = match value {
      RibirTouchPhase::Started => WinitTouchPhase::Started,
      RibirTouchPhase::Moved => WinitTouchPhase::Moved,
      RibirTouchPhase::Ended => WinitTouchPhase::Ended,
      RibirTouchPhase::Cancelled => WinitTouchPhase::Cancelled,
    };
    tb.into()
  }
}
