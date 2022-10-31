pub struct AnimateState<S> {
  init_fn: Box<dyn Fn() -> S>,
  final_fn: Box<dyn Fn() -> S>,
  write_fn: Box<dyn Fn(S)>,
}

impl<S> AnimateState<S> {
  #[inline]
  pub fn new(
    init_fn: Box<dyn Fn() -> S>,
    final_fn: Box<dyn Fn() -> S>,
    write_fn: Box<dyn Fn(S)>,
  ) -> Self {
    Self { init_fn, final_fn, write_fn }
  }

  #[inline]
  pub fn init_value(&self) -> S { (self.init_fn)() }
  #[inline]
  pub fn finial_value(&self) -> S { (self.final_fn)() }
  #[inline]
  pub fn update(&mut self, v: S) { (self.write_fn)(v) }
}
