use super::flex_item::*;
use super::vec_layouts::*;
use crate::prelude::*;

use crate::render::render_tree::*;
use crate::render::BoxLayout;
use crate::render::RenderObjectSafety;
use canvas::LogicUnit;
use std::marker::PhantomData;

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
  layouts: VecLayouts,
  pub bound: BoxLayout,
}

fn object_width(obj: &dyn RenderObjectSafety) -> Option<f32> {
  obj.get_size().map(|size| size.width)
}

fn object_height(obj: &dyn RenderObjectSafety) -> Option<f32> {
  obj.get_size().map(|size| size.height)
}

impl FlexContainer {
  pub fn new(axis: Axis, layout_type: LayoutConstraints) -> FlexContainer {
    FlexContainer {
      axis,
      layouts: VecLayouts::new(),
      bound: BoxLayout::new(layout_type),
    }
  }

  pub fn child_offset(&self, idx: usize) -> Option<Point> { self.layouts.position(idx) }
  pub fn main_size(&self, obj: &dyn RenderObjectSafety) -> Option<f32> {
    match self.axis {
      Axis::Horizontal => object_width(obj),
      Axis::Vertical => object_height(obj),
    }
  }

  pub fn cross_size(&self, obj: &dyn RenderObjectSafety) -> Option<f32> {
    match self.axis {
      Axis::Vertical => object_width(obj),
      Axis::Horizontal => object_height(obj),
    }
  }

  pub fn flex_layout<'a>(&mut self, id: RenderId, ctx: &mut RenderCtx<'a>) {
    let size = self.fix_child_size(id, ctx);

    self.fix_child_position(id, size, ctx);
    self.update_self_size(size);
  }

  fn update_self_size(&mut self, size: Size) {
    let real_size = match self.axis {
      Axis::Horizontal => size,
      Axis::Vertical => Size {
        width: size.height,
        height: size.width,
        _unit: PhantomData::<LogicUnit>,
      },
    };
    self.bound.set_size(Some(real_size));
  }

  fn fix_child_size<'a>(&self, id: RenderId, ctx: &mut RenderCtx<'a>) -> Size {
    let mut v = vec![];
    let mut autos = vec![];

    let bound = self.bound.get_box_limit();
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
        Axis::Horizontal => self.bound.get_box_limit().min_width,
        Axis::Vertical => self.bound.get_box_limit().min_height,
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
    self.layouts.reset(v.len());
    let mut x = 0.0;
    let mut y = 0.0;
    v.iter().enumerate().for_each(|(idx, value)| {
      self.layouts.update_size(
        idx,
        ctx
          .render_object(*value)
          .and_then(|obj| obj.get_size())
          .unwrap(),
      );
      self.layouts.update_position(idx, Point::new(x, y));
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
    if let Some(child) = ctx.render_object(id) {
      return (self.main_size(child), self.cross_size(child));
    }
    (None, None)
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
