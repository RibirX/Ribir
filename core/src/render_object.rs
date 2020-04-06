use crate::render_ctx::*;
use crate::render_object_box::{LayoutConstraints, RenderObjectBox};
use slab_tree::*;
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
  fn layout(&mut self, node_id: NodeId, ctx: &mut RenderCtx) {
    layout_sub_tree(ctx, node_id);
  }

  fn mark_dirty(&self, node_id: NodeId, ctx: &mut RenderCtx) {
    let mut id = ctx.get_render_box_id(node_id);
    if id.is_none() {
      return;
    }
    let mut constrants = mark_dirty_by_id(id.unwrap(), ctx);
    while constrants == LayoutConstraints::EffectedByBoth
      || constrants == LayoutConstraints::EffectedByChildren
    {
      let parent_id = ctx.get_parent_box_id(id.unwrap());
      if parent_id.is_none() {
        break;
      }
      id = parent_id;
      constrants = mark_dirty_by_id(id.unwrap(), ctx);
    }

    let mut ids = vec![];
    ctx.step_into_child_box_reverse(id.unwrap(), &mut ids);
    while ids.len() > 0 {
      let id = ids.pop().unwrap();
      let mut node = ctx.tree.get_mut(id).unwrap();
      let render_box = node.data().to_render_box().unwrap();
      constrants = render_box.get_constraints();
      if constrants == LayoutConstraints::EffectedByBoth
        || constrants == LayoutConstraints::EffectedByParent
      {
        render_box.mark_dirty();
        ctx.step_into_child_box_reverse(id, &mut ids);
      }
    }
  }
  fn to_render_box(&mut self) -> Option<&mut dyn RenderObjectBox> { None }
}

fn mark_dirty_by_id(id: NodeId, ctx: &mut RenderCtx) -> LayoutConstraints {
  let mut node = ctx.tree.get_mut(id).unwrap();
  let render_box = node.data().to_render_box().unwrap();
  render_box.mark_dirty();
  return render_box.get_constraints();
}

fn layout_sub_tree(ctx: &mut RenderCtx, node_id: NodeId) {
  let mut ids = vec![node_id];
  let mut down_ids = vec![];
  let mut_ptr = ctx as *mut RenderCtx;
  while ids.len() > 0 {
    let mut id = ids.pop().unwrap();
    id = ctx.get_render_box_id(id).unwrap();

    let mut node = ctx.tree.get_mut(id).unwrap();
    let render_box = node.data().to_render_box().unwrap();
    if !render_box.is_dirty() {
      continue;
    }

    // the context deliver in layout need a more elegant way
    unsafe {
      render_box.layout_sink(&mut *mut_ptr, id);
    }

    ctx.step_into_child_box_reverse(id, &mut ids);
    down_ids.push(id);
  }

  while down_ids.len() > 0 {
    let id = down_ids.pop().unwrap();
    let mut node = ctx.tree.get_mut(id).unwrap();
    let render_box = node.data().to_render_box().unwrap();
    unsafe {
      render_box.layout_bubble(&mut *mut_ptr, id);
    }
  }
}
