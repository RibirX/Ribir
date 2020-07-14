use crate::prelude::*;

/// Widget use to hold a place for single/multi child widget.
#[derive(Debug)]
pub struct PhantomWidget;

impl CombinationWidget for PhantomWidget {
  #[inline]
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget { unreachable!() }
}
