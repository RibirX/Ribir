use crate::prelude::*;

#[derive(Debug)]
pub struct PhantomWidget;

impl CombinationWidget for PhantomWidget {
  fn build(&self, _: &mut BuildCtx) -> BoxWidget { unreachable!() }
}
