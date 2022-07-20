mod animation_ctrl;
mod animation_progress;
mod animation_state;
mod animation_trigger;
mod animation_widget;
mod progress_state;
mod repeat_mode;
mod state_widget;
mod transition_widget;
mod tween;

pub use animation_ctrl::*;
pub use animation_trigger::*;
pub use animation_widget::*;
pub use progress_state::ProgressState;
pub use repeat_mode::RepeatMode;
pub use state_widget::*;
pub use transition_widget::*;
pub use tween::Tween;

use self::{animation_progress::new_animation_progress, animation_state::AnimationState};

// Transform the Region from [0.0 - 1.0] to [0.0 - 1.0]
// animation use the curve map to implement ease effect
pub trait Curve {
  fn transform(&self, t: f32) -> f32;
}
pub type CurveGenerator = Box<dyn Fn() -> Box<dyn Curve + 'static>>;

pub struct CurveLinear {}
impl Curve for CurveLinear {
  fn transform(&self, val: f32) -> f32 { val }
}
pub fn linear() -> Box<dyn Curve + 'static> { Box::new(CurveLinear {}) }
