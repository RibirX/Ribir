use std::convert::Infallible;
#[cfg(not(target_family = "wasm"))]
pub use std::time::{Duration, Instant};

use rxrust::prelude::{Observer, Subject};
#[cfg(target_family = "wasm")]
pub use web_time::{Duration, Instant};

/// Frame ticker emit message when new frame need to draw.
#[derive(Default, Clone)]
pub struct FrameTicker {
  subject: Subject<'static, FrameMsg, Infallible>,
}

/// Message emitted at different status of a frame.

#[derive(Clone)]
pub enum FrameMsg {
  /// This msg emit when all event has processed and framework begin to do
  /// layout & paint of the frame.
  ///
  /// only the first frame of continuous frames that do not need to be drawn
  /// will be sent this message.
  NewFrame(Instant),
  /// This Msg emit when performed layout finished, and widget tree ready to
  /// draw.
  /// #  Notice  
  /// - this message may emit more than once, if someone listen this message and
  ///   do some stuff to lead to some widget need relayout, be careful to modify
  ///   widget in the listener of this message.
  LayoutReady(Instant),
  /// This msg emit after render data has submitted that mean all stuffs of
  /// current frame need to processed by framework done.
  ///
  /// only the first frame of continuous frames that do not need to be drawn
  /// will be sent this message.
  Finish(Instant),
}

impl FrameTicker {
  #[inline]
  pub(crate) fn emit(&self, msg: FrameMsg) { self.subject.clone().next(msg) }

  #[inline]
  pub fn frame_tick_stream(&self) -> Subject<'static, FrameMsg, Infallible> { self.subject.clone() }
}
