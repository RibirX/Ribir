use super::{AnimationGenerator, Tween};
use std::cell::RefCell;
use std::rc::Rc;

pub struct AnimationValueTrigger<T>
where
  T: Clone + PartialEq + Tween + 'static,
{
  pub val: T,
  gen: Box<dyn FnMut(T) -> Option<Box<dyn FnMut(f32)>>>,
}

impl<T> AnimationValueTrigger<T>
where
  T: Clone + PartialEq + Tween + 'static,
{
  pub fn new<R>(mut init: T, target: R) -> Self
  where
    R: 'static + FnMut(T),
  {
    let wrap = Rc::new(RefCell::new(target));
    Self {
      val: init.clone(),
      gen: Box::new(move |val: T| -> Option<Box<dyn FnMut(f32)>> {
        if init == val {
          return None;
        }
        let begin = init.clone();
        let end = val.clone();
        let writer = wrap.clone();
        init = val;
        Some(Box::new(move |p| {
          (writer.borrow_mut())(Tween::tween(&begin, &end, p))
        }))
      }),
    }
  }
}

impl<T> AnimationGenerator for AnimationValueTrigger<T>
where
  T: Clone + PartialEq + Tween + 'static,
{
  fn animation(&mut self) -> Option<Box<dyn FnMut(f32)>> { (self.gen)(self.val.clone()) }
}
