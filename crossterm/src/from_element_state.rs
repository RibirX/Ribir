use crossterm::event::KeyEventKind as CrosstermElementState;
use ribir_core::prelude::ElementState as RibirElementState;

pub struct WrappedElementState(CrosstermElementState);

impl From<CrosstermElementState> for WrappedElementState {
  fn from(value: CrosstermElementState) -> Self { WrappedElementState(value) }
}

impl From<WrappedElementState> for CrosstermElementState {
  fn from(val: WrappedElementState) -> Self { val.0 }
}

impl From<WrappedElementState> for RibirElementState {
  fn from(val: WrappedElementState) -> Self {
    match val.0 {
      CrosstermElementState::Press => RibirElementState::Pressed,
      CrosstermElementState::Release => RibirElementState::Released,
      CrosstermElementState::Repeat => RibirElementState::Pressed,
    }
  }
}

// impl From<RibirElementState> for WrappedElementState {
//   fn from(value: RibirElementState) -> WrappedElementState {
//     let es = match value {
//       RibirElementState::Pressed => CrosstermElementState::Pressed,
//       RibirElementState::Released => CrosstermElementState::Released,
//     };
//     es.into()
//   }
// }
