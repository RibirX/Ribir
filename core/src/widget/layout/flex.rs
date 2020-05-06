// use super::box_constraint::*;
use super::flex_item::*;
use super::vec_layouts::*;
use crate::prelude::*;
// use crate::render::render_ctx::RenderCtx;
use crate::render::render_tree::*;
use crate::render::BoxLayout;
use crate::render::RenderObjectSafety;

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

fn object_width(obj: &dyn RenderObjectSafety) -> Option<f64> {
  return obj.get_size().map(|size| size.width);
}

fn object_height(obj: &dyn RenderObjectSafety) -> Option<f64> {
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

  pub fn main_size(&self, obj: &dyn RenderObjectSafety) -> Option<f64> {
    return match self.axis {
      Axis::Horizontal => object_width(obj),
      Axis::Vertical => object_height(obj),
    };
  }

  pub fn cross_size(&self, obj: &dyn RenderObjectSafety) -> Option<f64> {
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

    let bound = self.bound.get_box_bound();
    ctx.collect_children(id, &mut v);

    let mut total_flex = 0;
    let mut allocated: f64 = 0.0;
    let mut cross_size: f64 = 0.0;
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
    // assert!(total_flex != 0 && (constraints & EFFECTED_BY_CHILDREN));

    let size = self.size.clone().unwrap();
    let space = size.width - allocated;
    if total_flex > 0 {
      let s = space / f64::from(total_flex);

      for child_id in autos {
        let flex = ctx.render_object(child_id).unwrap().flex().unwrap();
        // child.main_axis(flex * s);
        if let (Some(main), Some(cross)) = self.child_layout(child_id, ctx) {
          allocated = allocated + main;
          cross_size = cross_size.max(cross);
        }
      }
    }

    return Size {
      width: allocated,
      height: cross_size,
    };
  }

  fn fix_child_position<'a>(
    &mut self,
    id: RenderId,
    content: Size,
    ctx: &mut RenderCtx<'a>,
  ) {
    let mut v = vec![];
    ctx.collect_children(id, &mut v);

    // todo fix child position
  }

  fn child_layout<'a>(
    &self,
    id: RenderId,
    ctx: &mut RenderCtx<'a>,
  ) -> (Option<f64>, Option<f64>) {
    ctx.perform_layout(id);
    if let Some(child) = ctx.render_object(id) {
      return (self.main_size(child), self.cross_size(child));
    }
    return (None, None);
  }
}
