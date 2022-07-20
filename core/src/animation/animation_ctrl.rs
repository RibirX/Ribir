use std::{cell::RefCell, rc::Rc};

use rxrust::{
  ops::box_it::LocalCloneBoxOp,
  prelude::{LocalSubject, MutRc, Observable, Observer, SubscribeNext},
  subscription::{SingleSubscription, SubscriptionGuard},
};

use crate::ticker::Ticker;

use super::{animation_state::AnimationState, AnimationObservable, Curve};

pub struct AnimationController {
  ticker: Ticker,
  state: Rc<RefCell<AnimationState>>,
  obser: LocalSubject<'static, f32, ()>,
  #[allow(dead_code)]
  guard: SubscriptionGuard<MutRc<SingleSubscription>>,
}

impl AnimationController {
  pub fn new(ticker: Ticker, state: Rc<RefCell<AnimationState>>, curve: Box<dyn Curve>) -> Self {
    let state2 = state.clone();
    let obser = LocalSubject::default();
    let mut obser2 = obser.clone();
    let guard = ticker
      .observable()
      .subscribe(move |t| {
        if let Some(state) = state2.borrow_mut().update(Some(t)) {
          obser2.next(curve.transform(state.val()));
        }
      })
      .unsubscribe_when_dropped();

    Self { ticker, state, obser, guard }
  }
}

impl AnimationObservable for AnimationController {
  fn observable(&mut self) -> LocalCloneBoxOp<'static, f32, ()> { self.obser.clone().box_it() }

  fn start(&mut self) {
    self.ticker.start();
    self.state.borrow_mut().start();
  }

  fn stop(&mut self) {
    self.ticker.stop();
    self.state.borrow_mut().stop();
  }
}
