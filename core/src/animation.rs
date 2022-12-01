pub mod easing;
mod progress;
mod state;
mod transition;

pub use easing::Easing;
pub use progress::AnimateProgress;
pub use state::*;
pub use transition::*;
mod animate;
pub use animate::*;
mod lerp;
pub use lerp::Lerp;
mod repeat;
pub use repeat::*;
mod property;
pub use property::{LerpProp, Prop, Property};
