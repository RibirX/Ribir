use std::ops::{Deref, DerefMut};
use std::{cell::RefCell, rc::Rc};

use rxrust::{observable::SubscribeNext, ops::box_it::LocalCloneBoxOp};

use crate::prelude::{
  compose_child_as_data_widget, BuildCtx, ComposeSingleChild, Query, Stateful, Widget,
};
use crate::ticker::Ticker;
use crate::{impl_query_self_only, prelude::*};

use super::AnimationTransition;

pub struct AnimationGeneratorHandle(*const dyn AnimationGenerator);

pub trait AnimationObservable {
  fn observable(&mut self) -> LocalCloneBoxOp<'static, f32, ()>;
  fn start(&mut self);
  fn stop(&mut self);
}

pub trait AnimationGenerator {
  fn animation(&mut self) -> Option<Box<dyn FnMut(f32)>>;
}

pub struct Animation<T: 'static>
where
  T: AnimationTransition,
{
  pub value: f32,

  ticker: Ticker,
  generators: Vec<Rc<RefCell<dyn AnimationGenerator>>>,
  ctrl_factory: T,
  ctrl: T::Observable,
  animation: Rc<RefCell<Option<Box<dyn FnMut(f32)>>>>,
}

impl<T: 'static> Animation<T>
where
  T: AnimationTransition,
{
  pub fn new(ctrl_factory: T, ctx: &mut BuildCtx) -> Stateful<Self> {
    let ticker = ctx.ticker();
    let ctrl = ctrl_factory.animation_ctrl(ticker.clone());
    let ins = Self {
      value: 1.,
      ticker,
      generators: Vec::default(),
      ctrl_factory,
      ctrl,

      animation: Rc::new(RefCell::new(None)),
    };

    let animation = ins.animation.clone();
    let stateful = ins.into_stateful();
    let stateful_clone = stateful.clone();

    let animation_wrap = Box::new(move |p: f32| {
      if animation.borrow().is_none() {
        *animation.borrow_mut() = Some(stateful_clone.shallow_ref().animation());
      }
      (animation.borrow_mut().as_mut().unwrap())(p);
    });

    stateful.state_change(|w| w.value).subscribe(move |c| {
      (animation_wrap)(c.after);
    });
    stateful
  }

  pub fn add_animation<G: AnimationGenerator + 'static>(&mut self, gen: G) -> Rc<RefCell<G>> {
    let wrap = Rc::new(RefCell::new(gen));
    self.generators.push(wrap.clone());
    wrap
  }

  fn animation(&mut self) -> Box<dyn FnMut(f32)> {
    let mut funcs = self.generators.iter_mut().fold(vec![], |mut collects, g| {
      if let Some(animation) = g.borrow_mut().animation() {
        collects.push(animation);
      }
      collects
    });

    Box::new(move |p: f32| {
      funcs.iter_mut().for_each(|f| {
        (*f)(p);
      });
    })
  }
}

impl<T> Deref for Animation<T>
where
  T: AnimationTransition,
{
  type Target = T::Observable;
  fn deref(&self) -> &Self::Target { &self.ctrl }
}

impl<T> DerefMut for Animation<T>
where
  T: AnimationTransition,
{
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ctrl }
}

impl<T> Query for Animation<T>
where
  T: AnimationTransition,
{
  impl_query_self_only!();
}

impl<T> ComposeSingleChild for Animation<T>
where
  T: AnimationTransition,
{
  #[inline]
  fn compose_single_child(
    this: Stateful<Self>,
    child: Option<Widget>,
    _ctx: &mut BuildCtx,
  ) -> Widget {
    compose_child_as_data_widget(child, this, |w| w)
  }
}

pub trait RetriggerAnimation {
  fn retrigger(&mut self);
}

impl<T> RetriggerAnimation for Stateful<Animation<T>>
where
  T: AnimationTransition,
{
  fn retrigger(&mut self) {
    let this_clone = self.clone();
    let mut this_ref = self.silent_ref();
    *this_ref.animation.borrow_mut() = None;
    this_ref.ctrl = this_ref
      .ctrl_factory
      .animation_ctrl(this_ref.ticker.clone());
    this_ref.ctrl.observable().subscribe(move |f| {
      this_clone.state_ref().value = f;
    });
    this_ref.ctrl.start();
  }
}
