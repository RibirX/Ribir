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
#[derive(Debug)]
pub enum Axis {
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

#[derive(Debug)]
pub struct FlexContainer {
  axis: Axis,
  size: Option<Size>,
  layouts: VecLayouts,
  pub bound: BoxLayout,
}

fn object_width(obj: &dyn RenderObjectSafety) -> Option<f32> {
  return obj.get_size().map(|size| size.width);
}

fn object_height(obj: &dyn RenderObjectSafety) -> Option<f32> {
  return obj.get_size().map(|size| size.height);
}

impl FlexContainer {
  pub fn new(axis: Axis, layout_type: LayoutConstraints) -> FlexContainer {
    return FlexContainer {
      axis: axis,
      size: None,
      layouts: VecLayouts::new(),
      bound: BoxLayout::new(layout_type),
    };
  }

  pub fn main_size(&self, obj: &dyn RenderObjectSafety) -> Option<f32> {
    return match self.axis {
      Axis::Horizontal => object_width(obj),
      Axis::Vertical => object_height(obj),
    };
  }

  pub fn cross_size(&self, obj: &dyn RenderObjectSafety) -> Option<f32> {
    return match self.axis {
      Axis::Vertical => object_width(obj),
      Axis::Horizontal => object_height(obj),
    };
  }

  pub fn flex_layout<'a>(&mut self, id: RenderId, ctx: &mut RenderCtx<'a>) {
    let size = self.fix_child_size(id, ctx);
    self.fix_child_position(id, size, ctx);
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
        total_flex = total_flex + flex;
        autos.push(child_id);
      } else {
        if let (Some(main), Some(cross)) = self.child_layout(child_id, ctx) {
          allocated = allocated + main;
          cross_size = cross_size.max(cross);
        }
      }
    }

    let size = self.size.clone().unwrap();
    let space = size.width - allocated;
    if total_flex > 0 {
      let s = space / (total_flex as f32);

      for child_id in autos {
        let flex = ctx.render_object(child_id).unwrap().flex().unwrap();
        ctx.set_box_limit(
          child_id,
          Some(self.set_main_to_bound(&bound, s * (flex as f32))),
        );

        if let (Some(main), Some(cross)) = self.child_layout(child_id, ctx) {
          allocated = allocated + main;
          cross_size = cross_size.max(cross);
        }
      }
    }

    return Size {
      width: allocated,
      height: cross_size,
      _unit: PhantomData::<LogicUnit>,
    };
  }

  fn fix_child_position<'a>(&mut self, id: RenderId, content: Size, ctx: &mut RenderCtx<'a>) {
    let mut v = vec![];
    ctx.collect_children(id, &mut v);

    let mut x = 0.0;
    let mut y = 0.0;
    for idx in 0..v.len() {
      self.layouts.update_size(
        idx,
        ctx
          .render_object(v[idx])
          .and_then(|obj| obj.get_size())
          .unwrap(),
      );
      self.layouts.update_position(
        idx,
        Point {
          x: x,
          y: y,
          _unit: PhantomData::<LogicUnit>,
        },
      );
      if let (Some(main), Some(cross)) = self.child_axis_size(v[idx], ctx) {
        match self.axis {
          Axis::Horizontal => x = x + main,
          Axis::Vertical => y = y + cross,
        }
      }
    }
  }

  fn child_layout<'a>(&self, id: RenderId, ctx: &mut RenderCtx<'a>) -> (Option<f32>, Option<f32>) {
    ctx.perform_layout(id);
    return self.child_axis_size(id, ctx);
  }

  fn child_axis_size<'a>(
    &self,
    id: RenderId,
    ctx: &mut RenderCtx<'a>,
  ) -> (Option<f32>, Option<f32>) {
    if let Some(child) = ctx.render_object(id) {
      return (self.main_size(child), self.cross_size(child));
    }
    return (None, None);
  }

  fn set_main_to_bound(&self, bound: &BoxLimit, main_size: f32) -> BoxLimit {
    let mut res = bound.clone();
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
    return res;
  }
}
