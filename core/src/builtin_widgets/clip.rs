use crate::prelude::*;

#[derive(Clone, Default)]
pub enum ClipType {
  #[default]
  Auto,
  Path(Path),
}

#[derive(SingleChild, Query, Clone, Declare)]
pub struct Clip {
  #[declare(default)]
  pub clip: ClipType,
}

impl Render for Clip {
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child_size = ctx.assert_perform_single_child_layout(clamp);
    match self.clip {
      ClipType::Auto => child_size,
      ClipType::Path(ref path) => path.bounds().max().to_tuple().into(),
    }
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let path = match &self.clip {
      ClipType::Auto => {
        let rect: lyon_geom::euclid::Rect<f32, LogicUnit> = Rect::from_size(
          ctx
            .box_rect()
            .expect("impossible without size in painting stage")
            .size,
        );
        Path::rect(&rect)
      }
      ClipType::Path(path) => path.clone(),
    };
    ctx.painter().clip(path);
  }
}
