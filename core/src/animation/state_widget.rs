use super::{AnimationGenerator, Tween};
use std::cell::RefCell;
use std::rc::Rc;

pub struct StateWidget<I, F, W, R>
where
  I: FnMut() -> R,
  F: FnMut() -> R,
  W: FnMut(R) + 'static,
  R: Tween + 'static,
{
  init_fn: I,
  final_fn: F,
  write_fn: Rc<RefCell<W>>,
}

impl<I, F, W, R> StateWidget<I, F, W, R>
where
  I: FnMut() -> R,
  F: FnMut() -> R,
  W: FnMut(R) + 'static,
  R: Tween + 'static,
{
  pub fn new(init_fn: I, final_fn: F, write_fn: W) -> Self {
    Self {
      init_fn,
      final_fn,
      write_fn: Rc::new(RefCell::new(write_fn)),
    }
  }
}

impl<I, F, W, R> AnimationGenerator for StateWidget<I, F, W, R>
where
  I: FnMut() -> R,
  F: FnMut() -> R,
  W: FnMut(R) + 'static,
  R: Tween + 'static,
{
  fn animation(&mut self) -> Option<Box<dyn FnMut(f32)>> {
    let init_val = (self.init_fn)();
    let final_val = (self.final_fn)();
    let write_fn = self.write_fn.clone();

    Some(Box::new(move |p| {
      (write_fn.borrow_mut())(Tween::tween(&init_val, &final_val, p));
    }))
  }
}
