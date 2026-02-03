use ribir_core::prelude::*;

use crate::layout::Direction;

#[derive(MultiChild)]
pub struct GridView {
  axis_dir: Direction,
  cross_axis_cnt: u32,
  /// The number of pixels from the leading edge of one tile to the trailing
  /// edge of the same tile in the main axis.
  child_x_extent: f32,
  /// The number of pixels from the leading edge of one tile to the trailing
  /// edge of the same tile in the cross axis.
  child_y_extent: f32,
  x_spacing: f32,
  y_spacing: f32,
}

impl GridView {
  #[inline]
  fn calc_child_pos(&self, idx: u32) -> Point {
    let main_offset = idx / self.cross_axis_cnt;
    let cross_offset = idx % self.cross_axis_cnt;
    match self.axis_dir {
      Direction::Vertical => Point::new(
        (cross_offset as f32) * (self.x_spacing + self.child_x_extent),
        (main_offset as f32) * (self.y_spacing + self.child_y_extent),
      ),
      Direction::Horizontal => Point::new(
        (main_offset as f32) * (self.x_spacing + self.child_x_extent),
        (cross_offset as f32) * (self.y_spacing + self.child_y_extent),
      ),
    }
  }

  #[inline]
  fn bound_size(&self, total_cnt: u32) -> Size {
    if total_cnt == 0 {
      return Size::new(0.0f32, 0.0f32);
    }
    let cross_cnt = total_cnt.min(self.cross_axis_cnt);
    let main_cnt = (total_cnt - 1) / self.cross_axis_cnt + 1;
    match self.axis_dir {
      Direction::Vertical => Size::new(
        (cross_cnt as f32) * (self.x_spacing + self.child_x_extent),
        (main_cnt as f32) * (self.y_spacing + self.child_y_extent) - self.y_spacing,
      ),
      Direction::Horizontal => Size::new(
        (main_cnt as f32) * (self.x_spacing + self.child_x_extent) - self.x_spacing,
        (cross_cnt as f32) * (self.y_spacing + self.child_y_extent),
      ),
    }
  }
}

impl Render for GridView {
  fn measure(&self, _: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let mut idx = 0;
    let (ctx, children) = ctx.split_children();
    for c in children {
      ctx.layout_child(
        c,
        BoxClamp {
          min: Size::new(self.child_x_extent, self.child_y_extent),
          max: Size::new(self.child_x_extent, self.child_y_extent),
        },
      );
      idx += 1;
    }

    self.bound_size(idx)
  }

  fn place_children(&self, _size: Size, ctx: &mut PlaceCtx) {
    let (ctx, children) = ctx.split_children();
    for (idx, c) in children.enumerate() {
      let pos = self.calc_child_pos(idx as u32);
      ctx.update_position(c, pos);
    }
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("gridView") }
}
