use std::time::Instant;

use rxrust::prelude::{LocalSubject, Observer};

/// Frame ticker emit message when new frame need to draw.
#[derive(Default)]
pub struct FrameTicker {
  subject: LocalSubject<'static, FrameMsg, ()>,
}

/// Message emitted at different status of a frame.

#[derive(Clone)]
pub enum FrameMsg {
  /// This msg emit when all event has processed and framework ready to do
  /// layout & paint.
  Ready(Instant),
  /// This msg emit after render data has submitted that mean all stuffs of
  /// current frame need to processed by framework done.
  Finish,
}

impl FrameTicker {
  #[inline]
  pub(crate) fn emit(&mut self, msg: FrameMsg) { self.subject.next(msg) }

  #[inline]
  pub fn frame_tick_stream(&self) -> LocalSubject<'static, FrameMsg, ()> { self.subject.clone() }
}
