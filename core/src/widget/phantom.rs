use crate::prelude::*;

#[derive(Debug, Widget)]
pub struct PhantomWidget;

impl CombinationWidget for PhantomWidget {
  fn build(&self, _: &mut BuildCtx) -> Box<dyn Widget> { unreachable!() }
}
