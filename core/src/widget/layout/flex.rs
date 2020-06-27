use super::flex_item::*;
use crate::prelude::*;

use crate::render::render_tree::*;

#[derive(Debug, Clone, Copy)]
pub enum FlexFit {
  Tight,
  Loose,
}
#[derive(Debug, Copy, Clone)]
pub enum Axis {
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

#[derive(Debug)]
pub struct FlexContainer {
  axis: Axis,
}

impl FlexContainer {
  pub fn new(axis: Axis, layout_type: LayoutConstraints) -> FlexContainer { FlexContainer { axis } }

  pub fn main_size(&self, id: RenderId, ctx: &RenderCtx) -> Option<f32> {
    let rect = ctx.box_rect(id);
    match self.axis {
      Axis::Horizontal => rect.map(|r| r.width()),
      Axis::Vertical => rect.map(|r| r.height()),
    }
  }

  pub fn cross_size(&self, id: RenderId, ctx: &RenderCtx) -> Option<f32> {
    let rect = ctx.box_rect(id);
    match self.axis {
      Axis::Vertical => rect.map(|r| r.width()),
      Axis::Horizontal => rect.map(|r| r.height()),
    }
  }

  pub fn flex_layout<'a>(&mut self, id: RenderId, ctx: &mut RenderCtx<'a>) -> Size {
    let size = self.fix_child_size(id, ctx);

    self.fix_child_position(id, size, ctx);
    let size = self.self_size(size);
    ctx.update_size(id, size);
    size
  }

  fn self_size(&mut self, size: Size) -> Size {
    match self.axis {
      Axis::Horizontal => size,
      Axis::Vertical => Size::new(size.height, size.width),
    }
  }

  fn fix_child_size<'a>(&self, id: RenderId, ctx: &mut RenderCtx<'a>) -> Size {
    let mut v = vec![];
    let mut autos = vec![];

    let bound = ctx.get_box_limit(id).unwrap_or(BoxUnLimit);
    ctx.collect_children(id, &mut v);

    let mut total_flex = 0;
    let mut allocated: f32 = 0.0;
    let mut cross_size: f32 = 0.0;
    for child_id in v {
      if let Some(flex) = ctx.render_object(child_id).and_then(|r| r.flex()) {
        total_flex += flex;
        autos.push(child_id);
      } else if let (Some(main), Some(cross)) = self.child_layout(child_id, ctx) {
        allocated += main;
        cross_size = cross_size.max(cross);
      }
    }

    if total_flex > 0 {
      let size = match self.axis {
        Axis::Horizontal => ctx.get_box_limit(id).unwrap_or(BoxUnLimit).min_width,
        Axis::Vertical => ctx.get_box_limit(id).unwrap_or(BoxUnLimit).min_height,
      };
      let space = size - allocated;
      let s = space / (total_flex as f32);

      for child_id in autos {
        let flex = ctx.render_object(child_id).unwrap().flex().unwrap();
        ctx.set_box_limit(
          child_id,
          Some(self.set_main_to_bound(&bound, s * (flex as f32))),
        );

        if let (Some(main), Some(cross)) = self.child_layout(child_id, ctx) {
          allocated += main;
          cross_size = cross_size.max(cross);
        }
      }
    }

    Size::new(allocated, cross_size)
  }

  fn fix_child_position<'a>(&mut self, id: RenderId, _content: Size, ctx: &mut RenderCtx<'a>) {
    let mut v = vec![];
    ctx.collect_children(id, &mut v);
    let mut x = 0.0;
    let mut y = 0.0;
    v.iter().for_each(|value| {
      ctx.update_child_pos(*value, Point::new(x, y));
      if let (Some(main), Some(_)) = self.child_axis_size(*value, ctx) {
        match self.axis {
          Axis::Horizontal => x += main,
          Axis::Vertical => y += main,
        }
      }
    })
  }

  fn child_layout<'a>(&self, id: RenderId, ctx: &mut RenderCtx<'a>) -> (Option<f32>, Option<f32>) {
    ctx.perform_layout(id);
    self.child_axis_size(id, ctx)
  }

  fn child_axis_size<'a>(
    &self,
    id: RenderId,
    ctx: &mut RenderCtx<'a>,
  ) -> (Option<f32>, Option<f32>) {
    return (self.main_size(id, ctx), self.cross_size(id, ctx));
  }

  fn set_main_to_bound(&self, bound: &BoxLimit, main_size: f32) -> BoxLimit {
    let mut res = *bound;
    match self.axis {
      Axis::Horizontal => {
        res.min_width = 0.0;
        res.max_width = main_size;
      }
      Axis::Vertical => {
        res.min_height = 0.0;
        res.max_height = main_size;
      }
    }
    res
  }
}
