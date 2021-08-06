use crate::prelude::*;

pub struct PhantomWidget;

impl CombinationWidget for PhantomWidget {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget { unreachable!() }
}
