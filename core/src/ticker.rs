use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use rxrust::prelude::{LocalSubject, Observer};

pub struct Ticker {
  subject: LocalSubject<'static, Instant, ()>,
  active: Rc<RefCell<bool>>,
}

impl Clone for Ticker {
  fn clone(&self) -> Self {
    Self {
      subject: self.subject.clone(),
      active: self.active.clone(),
    }
  }
}

impl Ticker {
  fn new() -> Self {
    Self {
      subject: LocalSubject::default(),
      active: Rc::new(RefCell::new(false)),
    }
  }

  pub fn observable(&self) -> LocalSubject<'static, Instant, ()> { self.subject.clone() } // &impl Observable<Item = Instant, Err = ()> { &self.subject }
  pub fn is_active(&self) -> bool { *self.active.borrow() }
  pub fn start(&mut self) { *self.active.borrow_mut() = true; }
  pub fn stop(&mut self) { *self.active.borrow_mut() = false; }

  fn trigger(&mut self, t: &Instant) -> bool {
    if !self.is_active() {
      return false;
    }
    self.subject.next(*t);
    true
  }

  fn clone(this: &Self) -> Self {
    Self {
      subject: this.subject.clone(),
      active: this.active.clone(),
    }
  }

  fn is_destroy(&self) -> bool { Rc::strong_count(&self.active) <= 1 }
}

/// from TickerProvider you can get the TickerCtrl. The application will trigger
/// the TickerCtrl every drawframe
pub struct TickerProvider {
  tickers: Vec<Ticker>,
}

impl Default for TickerProvider {
  fn default() -> Self { Self { tickers: Vec::new() } }
}

impl TickerProvider {
  pub fn ticker(&mut self) -> Ticker {
    let ticker = Ticker::new();
    self.tickers.push(Ticker::clone(&ticker));
    ticker
  }
  pub fn trigger(&mut self) -> bool {
    let now = Instant::now();
    let mut has_trigger = false;
    self
      .tickers
      .drain_filter(|ticker| {
        if !ticker.is_destroy() {
          has_trigger |= ticker.trigger(&now);
          false
        } else {
          true
        }
      })
      .for_each(drop);
    return has_trigger;
  }
}
