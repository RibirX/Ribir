use ribir::animation::TickerAnimationCtrl;
use ribir::animation::TickerProvider;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use std::time::Duration;
use std::time::Instant;

use crate::ticker_animation_ctrl::new_ticker_handle;
use crate::ticker_ctrl::RawTickerCtrl;

struct TickerMgr {
  tickers: Vec<Weak<RefCell<RawTickerCtrl>>>,
}

impl TickerMgr {
  fn new() -> Self { TickerMgr { tickers: Vec::default() } }
}

impl TickerProvider for TickerMgr {
  fn trigger(&mut self) -> bool {
    let now = Instant::now();

    let mut has_trigger = false;
    self
      .tickers
      .drain_filter(|ticker| {
        if let Some(ticker) = ticker.upgrade() {
          ticker.borrow_mut().update(Some(now));
          has_trigger = true;
          false
        } else {
          true
        }
      })
      .for_each(drop);
    return has_trigger;
  }

  fn ticker_ctrl(&mut self, duration: Duration) -> Box<dyn TickerAnimationCtrl> {
    let handle = new_ticker_handle(duration);
    self.tickers.push(Rc::downgrade(&handle.0));
    Box::new(handle)
  }
}

pub fn new_ticker_animation_mgr() -> Box<dyn TickerProvider> { Box::new(TickerMgr::new()) }

#[cfg(test)]
mod tests {
  use super::TickerMgr;
  use ribir::animation::TickerProvider;
  use std::time::Duration;

  #[test]
  fn test_ticker_drop() {
    let mut mgr = TickerMgr::new();
    {
      let mut vec = Vec::default();

      vec.push(mgr.ticker_ctrl(Duration::from_millis(100)));
      vec.push(mgr.ticker_ctrl(Duration::from_millis(100)));

      mgr.trigger();
      assert!(mgr.tickers.len() == 2);

      vec.pop();
      mgr.trigger();
    }

    assert!(mgr.tickers.len() == 1);
    mgr.trigger();
    assert!(mgr.tickers.len() == 0);
  }
}
