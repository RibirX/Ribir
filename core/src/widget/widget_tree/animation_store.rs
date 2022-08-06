use crate::{
  prelude::{AnimateProgress, AnimationCtrl},
  ticker::FrameMsg,
};
use algo::id_map::{Id, IdMap};
use rxrust::{
  prelude::{LocalSubject, MutRc, SubscribeNext},
  subscription::{SingleSubscription, SubscriptionGuard},
};

use super::WidgetTree;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct AnimationId(Id);

pub struct AnimationStore {
  animations: Rc<RefCell<IdMap<Box<dyn AnimationCtrl>>>>,
  running: Rc<RefCell<HashSet<AnimationId, ahash::RandomState>>>,
  frame_ticker: LocalSubject<'static, FrameMsg, ()>,
  tick_msg_guard: Option<SubscriptionGuard<MutRc<SingleSubscription>>>,
}

impl AnimationId {
  pub fn start(self, mgr: &mut AnimationStore) {
    let AnimationStore {
      animations,
      running,
      frame_ticker,
      tick_msg_guard,
    } = mgr;
    running.borrow_mut().insert(self);

    if tick_msg_guard.is_none() {
      let runnings = running.clone();
      let animations = animations.clone();
      let guard = frame_ticker
        .clone()
        .subscribe(move |msg| match msg {
          FrameMsg::Ready(time) => {
            let mut animations = animations.borrow_mut();
            let mut runnings = runnings.borrow_mut();

            let mut finished = vec![];
            runnings.iter().for_each(|id| {
              let p = id.running_animation(&mut *animations).lerp_by(time);
              if matches!(p, AnimateProgress::Finish) {
                finished.push(*id);
              }
            });
            finished.iter().for_each(|id| {
              runnings.remove(id);
            });
          }
          FrameMsg::Finish => {
            let mut animations = animations.borrow_mut();
            let runnings = runnings.borrow_mut();

            runnings
              .iter()
              .for_each(|id| id.running_animation(&mut *animations).frame_finished());
          }
        })
        .unsubscribe_when_dropped();
      *tick_msg_guard = Some(guard)
    }
  }

  pub fn stop(self, mgr: &mut AnimationStore) {
    mgr.running.borrow_mut().remove(&self);
    // if there isn't running  animation, cancel the ticker subscription.
    if mgr.running.borrow().is_empty() {
      mgr.tick_msg_guard.take();
    }
  }

  #[inline]
  pub fn is_running(self, mgr: &mut AnimationStore) -> bool { mgr.running.borrow().contains(&self) }

  pub fn drop(self, mgr: &mut AnimationStore) {
    mgr.animations.borrow_mut().remove(self.0);
    mgr.running.borrow_mut().remove(&self);
  }

  fn running_animation(
    self,
    animations: &mut IdMap<Box<dyn AnimationCtrl>>,
  ) -> &mut dyn AnimationCtrl {
    &mut **animations
      .get_mut(self.0)
      .expect("Running animation not found.")
  }
}

impl AnimationStore {
  pub fn new(frame_ticker: LocalSubject<'static, FrameMsg, ()>) -> Self {
    Self {
      animations: <_>::default(),
      running: <_>::default(),
      frame_ticker,
      tick_msg_guard: None,
    }
  }

  pub fn register(&mut self, animation: Box<dyn AnimationCtrl>) -> AnimationId {
    AnimationId(self.animations.borrow_mut().insert(animation))
  }
}

impl WidgetTree {
  pub fn register_animate(&mut self, animate: Box<dyn AnimationCtrl>) -> AnimationId {
    self.animations_store.register(animate)
  }
}
