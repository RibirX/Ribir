use crate::prelude::*;

use super::scroll_view::ScrollInfo;
#[derive(Declare, Clone, SingleChildWidget)]
struct ScrollBarTrack {
  layout: ScrollInfo,
  #[declare(default = "ctx.theme().scrollbar.track_width")]
  cross_width: f32,
}

impl ScrollBarTrack {
  fn offset(&self) -> Point {
    let offset = self.layout.scrollbar_offset();
    match self.layout.direction() {
      Direction::Horizontal => Point::new(-offset, 0.),
      Direction::Vertical => Point::new(0., -offset),
    }
  }

  fn size(&self, clamp: BoxClamp) -> Size {
    match self.layout.direction() {
      Direction::Horizontal => Size::new(clamp.max.width, self.cross_width),
      Direction::Vertical => Size::new(self.cross_width, clamp.max.height),
    }
  }
}

impl Render for ScrollBarTrack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if !self.layout.is_show() {
      return Size::default();
    }
    let child = ctx
      .single_render_child()
      .expect("Margin must have one child");
    let size = self.size(clamp);
    ctx.perform_render_child_layout(child, clamp);
    ctx.update_position(child, self.offset());
    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[derive(Declare, Clone)]
pub struct ScrollBarThumb {
  layout: ScrollInfo,
  cross_width: f32,
}

impl Render for ScrollBarThumb {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    if !self.layout.is_show() {
      return Size::default();
    }
    let bar_size = self.layout.scrollbar_size();
    match self.layout.direction() {
      Direction::Horizontal => Size::new(bar_size, self.cross_width),
      Direction::Vertical => Size::new(self.cross_width, bar_size),
    }
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[derive(Declare, Clone)]
pub struct ScrollBar {
  pub info: ScrollInfo,

  #[declare(default = "ctx.theme().scrollbar.track_box.clone()")]
  pub track_box: ScrollBoxDecorationStyle,

  #[declare(default = "ctx.theme().scrollbar.track_width")]
  pub track_width: f32,

  #[declare(default = "ctx.theme().scrollbar.thumb_box.clone()")]
  pub thumb_box: ScrollBoxDecorationStyle,

  #[declare(default = "ctx.theme().scrollbar.thumb_width")]
  pub thumb_width: f32,
}

impl ScrollBar {
  // todo: remove me after scroll view build from declare
  pub fn new(info: ScrollInfo, ctx: &mut BuildCtx) -> Self {
    ScrollBar {
      info,
      track_box: ctx.theme().scrollbar.track_box.clone(),
      track_width: ctx.theme().scrollbar.track_width,
      thumb_box: ctx.theme().scrollbar.thumb_box.clone(),
      thumb_width: ctx.theme().scrollbar.thumb_width,
    }
  }
}

impl Compose for ScrollBar {
  #[widget]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare ScrollBarTrack {
        layout: self.info.clone(),
        cross_width: self.track_width,
        background: self.track_box.background.clone(),

        ScrollBarThumb {
          layout: self.info.clone(),
          cross_width: self.thumb_width,
          background: self.thumb_box.background.clone(),
        }
      }
    }
  }
}
