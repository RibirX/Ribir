use crate::prelude::*;

/// A widget that clips its child using a specified path.
///
/// # Example
///
/// Clip a container by a rounded rectangle path.
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Clip {
///     clip_path: Path::rect_round(&Rect::new(Point::zero(), Size::new(50., 50.)), &Radius::all(25.)),
///     @Container {
///       size: Size::new(100., 100.),
///       background: Color::RED
///     }
///   }
/// };
/// ```
#[derive(SingleChild, Declare)]
pub struct Clip {
  pub clip_path: Path,
}

impl Render for Clip {
  fn size_affected_by_child(&self) -> bool { false }

  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    ctx.perform_single_child_layout(clamp);
    self
      .clip_path
      .bounds(None)
      .max()
      .to_tuple()
      .into()
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().clip(self.clip_path.clone().into()); }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let clip_rect = self.clip_path.bounds(None);
    ctx.clip(clip_rect);
    Some(clip_rect)
  }
}
