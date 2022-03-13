mod progress_state;
mod repeat_mode;
use std::time::Duration;

pub use progress_state::ProgressState;
pub use repeat_mode::RepeatMode;
use rxrust::ops::box_it::LocalBoxOp;

/// the ctrl handle return by TickerRunningCtrl.listen
/// after dispose is call, the call_back would'n be call again
pub trait TickerRunningHandle {
  fn dispose(&mut self);
}

pub trait TickerRunningCtrl {
  fn state(&self) -> ProgressState;
  fn reverse(&mut self);
  fn run(&mut self);
  fn pause(&mut self);
  fn is_run(&self) -> bool;
  fn is_complete(&self) -> bool;
  fn restart(&mut self, run: bool);

  /// the call_back will be call every ticker frame when running
  fn listen(&mut self, call_back: Box<dyn FnMut(ProgressState)>) -> Box<dyn TickerRunningHandle>;
}

/// you can listen the ticker signal by the TickerCtrl, the ticker will be stop
/// after the TickerCtrl droped
pub trait TickerAnimationCtrl: TickerRunningCtrl {
  fn with_repeat(self: Box<Self>, mode: RepeatMode) -> Box<dyn TickerAnimationCtrl>;
  fn with_round(self: Box<Self>) -> Box<dyn TickerAnimationCtrl>;
  fn span_secs(&self) -> f32;
}

/// A controller for an animation. AnimationCtrl produces values that range from
/// 0.0 to 1.0
pub trait AnimationCtrl {
  /// return the current progress, we control the progress to change the
  /// animation.
  fn state(&self) -> ProgressState;

  /// the value follow the progress changed, the animation subject the value to
  /// change.
  fn value(&self) -> f32;

  /// from subject animation can observe the value when progress change
  fn subject(&mut self) -> LocalBoxOp<'static, f32, ()>;

  fn step(&mut self, step: f32);

  fn update_to(&mut self, state: ProgressState);
}

/// from TickerProvider you can get the TickerCtrl. The application will trigger
/// the TickerCtrl every drawframe
pub trait TickerProvider {
  /// trigger the TickerCtrl tick.
  fn trigger(&mut self) -> bool;

  fn ticker_ctrl(&mut self, duration: Duration) -> Box<dyn TickerAnimationCtrl>;
}
