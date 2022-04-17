use std::{cell::RefCell, rc::Rc, time::Duration};

use ribir::animation::{
  ProgressState, RepeatMode, TickerAnimationCtrl, TickerRunningCtrl, TickerRunningHandle,
};

use crate::ticker_ctrl::RawTickerCtrl;

pub(crate) struct TickerHandle(pub(crate) Rc<RefCell<RawTickerCtrl>>);

impl TickerRunningCtrl for TickerHandle {
  fn state(&self) -> ProgressState { self.0.borrow().state() }

  fn reverse(&mut self) { self.0.borrow_mut().reverse() }

  fn start(&mut self) { self.0.borrow_mut().start(); }

  fn pause(&mut self) { self.0.borrow_mut().pause(); }

  fn is_run(&self) -> bool { self.0.borrow().is_run() }

  fn is_complete(&self) -> bool { self.0.borrow().is_complete() }

  fn restart(&mut self, run: bool) { self.0.borrow_mut().restart(run); }

  fn listen(&mut self, f: Box<dyn FnMut(ProgressState)>) -> Box<dyn TickerRunningHandle> {
    self.0.borrow_mut().listen(f)
  }

  fn force_done(&mut self) { self.0.borrow_mut().force_done() }
}

impl TickerAnimationCtrl for TickerHandle {
  fn with_repeat(self: Box<Self>, mode: RepeatMode) -> Box<dyn TickerAnimationCtrl> {
    self.0.borrow_mut().with_repeat(mode);
    self
  }

  fn with_round(self: Box<Self>) -> Box<dyn TickerAnimationCtrl> {
    self.0.borrow_mut().with_round();
    self
  }

  fn span_secs(&self) -> f32 { self.0.borrow().span_secs() }
}

pub(crate) fn new_ticker_handle(duration: Duration) -> TickerHandle {
  TickerHandle(Rc::new(RefCell::new(RawTickerCtrl::new(duration))))
}
