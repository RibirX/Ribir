pub struct AnimationState<I, F, W> {
  init_fn: I,
  final_fn: F,
  write_fn: W,
}

impl<I, F, W, R> AnimationState<I, F, W>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
{
  #[inline]
  pub fn new(init_fn: I, final_fn: F, write_fn: W) -> Self { Self { init_fn, final_fn, write_fn } }

  #[inline]
  pub fn init_value(&self) -> R { (self.init_fn)() }
  #[inline]
  pub fn finial_value(&self) -> R { (self.final_fn)() }
  #[inline]
  pub fn update(&mut self, v: R) { (self.write_fn)(v) }
}
