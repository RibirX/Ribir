use crate::prelude::*;

impl RenderWidget for Path {
  type RO = Self;

  fn create_render_object(&self) -> Self::RO { self.clone() }

  fn update_render_object(&self, _: &mut Self::RO, _: &mut UpdateCtx) {
    unreachable!("As a stateless widget, impossible to call this method ")
  }
}

impl RenderObject for Path {
  fn perform_layout(&mut self, clamp: BoxClamp, _: &mut RenderCtx) -> Size {
    let size = self.box_rect().max().to_tuple().into();
    clamp.clamp(size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) { todo!() }
}
