use crate::{impl_query_self_only, prelude::*};

#[derive(Clone, Default)]
pub enum ClipType {
  #[default]
  Auto,
  Path(Path),
}

#[derive(SingleChild, Clone, Declare)]
pub struct Clip {
  #[declare(default)]
  pub clip: ClipType,
}

impl Render for Clip {
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = ctx.single_child().expect("Clip must have one child.");
    let child_size = ctx.perform_child_layout(child, clamp);
    match self.clip {
      ClipType::Auto => child_size,
      ClipType::Path(ref path) => path.box_rect().max().to_tuple().into(),
    }
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let path = match &self.clip {
      ClipType::Auto => {
        let rect = ctx
          .box_rect()
          .expect("impossible without size in painting stage");
        Path::rect(&rect, PathStyle::Fill)
      }
      ClipType::Path(path) => path.clone(),
    };
    ctx.painter().clip(path.clone());
  }
}

impl Query for Clip {
  impl_query_self_only!();
}
