use crate::{
  prelude::{AnimateCtrl, AnimateProgress},
  ticker::FrameMsg,
};
use algo::id_map::{Id, IdMap};
use rxrust::{
  prelude::{LocalSubject, MutRc, SubscribeNext},
  subscription::{SingleSubscription, SubscriptionGuard},
};

use super::WidgetTree;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Clone)]
pub struct AnimateHandler {
  id: Id,
  store: Rc<RefCell<AnimateStore>>,
}

pub struct AnimateStore {
  animations: IdMap<Box<dyn AnimateCtrl>>,
  running: HashSet<Id, ahash::RandomState>,
  frame_ticker: LocalSubject<'static, FrameMsg, ()>,
  tick_msg_guard: Option<SubscriptionGuard<MutRc<SingleSubscription>>>,
}

impl AnimateHandler {
  pub fn start(&self) {
    let c_store = self.store.clone();
    let mut store = self.store.borrow_mut();
    store.running.insert(self.id);
    if store.tick_msg_guard.is_none() {
      let guard = store
        .frame_ticker
        .clone()
        .subscribe(move |msg| {
          let mut store = c_store.borrow_mut();
          match msg {
            FrameMsg::Ready(time) => {
              let mut finished = vec![];
              store.inspect_running_animate(|id, animate| {
                let p = animate.lerp_by(time);
                if matches!(p, AnimateProgress::Finish) {
                  finished.push(id);
                }
              });

              finished.iter().for_each(|id| {
                store.running.remove(id);
              });
            }
            FrameMsg::Finish => {
              store.inspect_running_animate(|_, animate| animate.frame_finished())
            }
          }
        })
        .unsubscribe_when_dropped();
      store.tick_msg_guard = Some(guard)
    }
  }

  pub fn stop(&self) {
    let mut store = self.store.borrow_mut();
    store.running.remove(&self.id);
    // if there isn't running  animation, cancel the ticker subscription.
    if store.running.is_empty() {
      store.tick_msg_guard.take();
    }
  }

  #[inline]
  pub fn is_running(self) -> bool { self.store.borrow().running.contains(&self.id) }

  #[inline]
  pub fn unregister(self) -> Option<Box<dyn AnimateCtrl>> {
    let mut store = self.store.borrow_mut();
    store.running.remove(&self.id);
    store.animations.remove(self.id)
  }
}

impl AnimateStore {
  pub fn new(frame_ticker: LocalSubject<'static, FrameMsg, ()>) -> Self {
    Self {
      animations: <_>::default(),
      running: <_>::default(),
      frame_ticker,
      tick_msg_guard: None,
    }
  }

  pub fn register(this: Rc<RefCell<Self>>, animate: Box<dyn AnimateCtrl>) -> AnimateHandler {
    let id = this.borrow_mut().animations.insert(animate);
    AnimateHandler { id, store: this }
  }

  fn inspect_running_animate(&mut self, mut f: impl FnMut(Id, &mut dyn AnimateCtrl)) {
    self.running.iter().for_each(|id| {
      let animate = &mut **self.animations.get_mut(*id).expect(" Animate not found.");
      f(*id, animate)
    });
  }
}

impl WidgetTree {
  pub fn register_animate(&mut self, animate: Box<dyn AnimateCtrl>) -> AnimateHandler {
    AnimateStore::register(self.animations_store.clone(), animate)
  }
}
