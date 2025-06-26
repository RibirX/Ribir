use std::convert::Infallible;
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Duration, Instant};

use rxrust::prelude::Subject;
#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant};

/// Frame ticker emit message when new frame need to draw.
pub type FrameTicker = Subject<'static, FrameMsg, Infallible>;

/// Message emitted at different status of a frame.

#[derive(Clone, Debug)]
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
