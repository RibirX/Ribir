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
  /// This message is emitted when all events have been processed and the
  /// framework begins the layout and painting of the frame.
  ///
  /// Only the first frame of continuous frames that do not require drawing will
  /// receive this message.
  NewFrame(Instant),
  /// This message is emitted before the framework starts the layout of the
  /// frame.
  BeforeLayout(Instant),
  /// This message is emitted when the layout process is completed, and the
  /// widget tree is ready to be rendered. # Notice
  /// - This message may be emitted more than once if there are listeners
  ///   performing actions that trigger a widget relayout. Exercise caution when
  ///   modifying widgets in the listener of this message.
  LayoutReady(Instant),
  /// This message is emitted after the render data has been submitted,
  /// indicating that all tasks for the current frame have been completed by the
  /// framework.
  ///
  /// Only the first frame of continuous frames that do not require drawing will
  /// receive this message.
  Finish(Instant),
}

impl FrameTicker {
  #[inline]
  pub(crate) fn emit(&self, msg: FrameMsg) { self.subject.clone().next(msg) }

  #[inline]
  pub fn frame_tick_stream(&self) -> Subject<'static, FrameMsg, Infallible> { self.subject.clone() }
}
