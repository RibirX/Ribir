use ribir_core::{impl_query_self_only, prelude::*};

#[derive(Clone)]
pub enum ClipType {
  Auto,
  Path(Path),
}

#[derive(SingleChild, Clone, Declare)]
pub struct Clip {
  clip: ClipType,
}

impl Render for Clip {
  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = ctx.single_child().expect("Clip must have one child.");
    ctx.perform_child_layout(child, clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let path = match &self.clip {
      ClipType::Auto => {
        let size = ctx
          .box_rect()
          .expect("impossible without size in painting stage");
        let mut builder = Path::builder();
        builder
          .begin_path(Point::zero())
          .line_to(Point::new(size.width(), 0.))
          .line_to(Point::new(size.width(), size.height()))
          .line_to(Point::new(0., size.height()))
          .end_path(true);
        builder.fill()
      }
      ClipType::Path(path) => path.clone(),
    };
    ctx.painter().clip(path.clone());
  }
}

impl Query for Clip {
  impl_query_self_only!();
}
