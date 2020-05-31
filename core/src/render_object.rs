use crate::{
  render_ctx::*,
  render_object_box::{LayoutConstraints, RenderObjectBox},
};
use indextree::*;
use std::fmt::Debug;

pub trait RenderObject: Debug {
  fn paint(&self);

  /// Layout flow is like message bubbling.
  /// It starts from the sub tree root, recursion deliver to each child
  /// RenderObject. There may have two opportunities invoking time each
  /// RenderObjectBox in the processing of layout. First time, the
  /// layout_sink, which will be called only when the RenderObjectBox's
  /// LayoutConstraints is EffectedByParent or EffectedByBoth, and it promise
  /// all the ancestors with LayoutConstraints of EffectedByParent or
  /// EffectedByBoth has called layout_sink. Second time, the layout_bubble,
  /// it promise all the children has called layout_bubble'. When the layout
  /// just decided by self or by parent, it should set it's bound in
  /// layout_sink,then the child with EffectedByParent after can get parent's
  /// bound otherwise should set in layout_bubble, when all the children's
  /// bound have been decided.
  fn perform_layout(&mut self, node_id: NodeId, ctx: &mut RenderCtx) {
    let box_id = ctx
      .get_render_box_id(node_id)
      .expect("perform_layout must under layout_box node");

    if !ctx.is_layout_dirty(&box_id) {
      return;
    }

    ctx.perform_layout_sink(box_id);

    let mut ids = vec![];
    ctx.collect_children_box(node_id, &mut ids);

    for id in ids {
      ctx.perform_layout(id);
    }

    ctx.perform_layout_bubble(box_id);
    ctx.clear_layout_dirty(&box_id);
  }

  fn mark_dirty(&self, node_id: NodeId, ctx: &mut RenderCtx) {
    let mut id = ctx.get_render_box_id(node_id);
    if id.is_none() {
      return;
    }
    loop {
      mark_dirty_down(id.unwrap(), ctx);
      let parent_id = ctx.get_parent_box_id(id.unwrap());
      if parent_id.is_none() {
        break;
      }
      let constraints = ctx.get_layout_constraints(parent_id.unwrap()).unwrap();
      if !constraints.contains(LayoutConstraints::EFFECTED_BY_CHILDREN) {
        break;
      }
      id = parent_id;
    }
    ctx.add_layout_sub_tree(id.unwrap());
  }
  fn to_render_box(&self) -> Option<&dyn RenderObjectBox> { None }

  fn to_render_box_mut(&mut self) -> Option<&mut dyn RenderObjectBox> { None }
}

fn mark_constraints_dirty(id: NodeId, ctx: &mut RenderCtx, target: LayoutConstraints) -> bool {
  if let Some(constraints) = ctx.get_layout_constraints(id) {
    if constraints.contains(target) {
      ctx.mark_layout_dirty(id);
      return true;
    }
  }
  false
}

fn mark_dirty_down(mut id: NodeId, ctx: &mut RenderCtx) {
  if let Some(box_id) = ctx.get_render_box_id(id) {
    if ctx.is_layout_dirty(&box_id) {
      return;
    }
    let mut ids = vec![];
    ctx.collect_children_box(id, &mut ids);
    while ids.len() > 0 {
      id = ids.pop().unwrap();
      if mark_constraints_dirty(id, ctx, LayoutConstraints::EFFECTED_BY_PARENT) {
        ctx.collect_children_box(id, &mut ids);
      }
    }
    ctx.mark_layout_dirty(box_id);
  }
}
