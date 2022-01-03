use super::layout_store::{BoxLayout, LayoutStore};
use crate::render::render_tree::*;
use crate::render::*;

/// A place to compute the render object's layout. Rather than holding children
/// directly, `RenderObject` perform layout across `RenderCtx`. `RenderCtx`
/// provide method to perform layout and also provides methods to access the
/// `RenderCtx` of the children.
pub struct RenderCtx<'a> {
  r_tree: &'a mut RenderTree,
  /// current render object id.
  rid: RenderId,
  layout_store: &'a mut LayoutStore,
}

impl<'a> RenderCtx<'a> {
  #[inline]
  pub(crate) fn new(
    rid: RenderId,
    tree: &'a mut RenderTree,
    layout_store: &'a mut LayoutStore,
  ) -> RenderCtx<'a> {
    RenderCtx { r_tree: tree, rid, layout_store }
  }

  /// Return the boxed rect of the RenderObject already placed.
  #[inline]
  pub fn box_rect(&self) -> Option<Rect> { self.layout_store.layout_box_rect(self.rid) }

  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_child_position(&mut self, child: RenderId, pos: Point) {
    debug_assert!(self.is_child(child));
    self.layout_store.layout_box_rect_mut(child).origin = pos;
  }

  /// Update the size of the child render object should place. Use this method
  /// to directly change the size of a render object, in most cast you needn't
  /// call this method, use  clamp to limit the child size is enough. Use this
  /// method only it you know what you are doing.

  #[inline]
  pub fn update_child_size(&mut self, child: RenderId, size: Size) {
    debug_assert!(self.is_child(child));
    self.layout_store.layout_box_rect_mut(child).size = size;
  }

  /// Return the boxed rect of the child render object already placed.
  #[inline]
  pub fn child_box_rect(&self, child: RenderId) -> Option<Rect> {
    debug_assert!(self.is_child(child));
    self.layout_store.layout_box_rect(child)
  }

  /// Do the work of computing the layout for child object, and return the
  /// render object box size. Should called from parent.
  pub fn perform_child_layout(&mut self, child: RenderId, clamp: BoxClamp) -> Size {
    debug_assert!(self.is_child(child));
    let rid = self.rid;
    self.rid = child;
    let size = self.perform_layout(clamp);
    self.rid = rid;
    size
  }

  /// Return the single child, panic if have more than once child.
  pub fn single_child(&mut self) -> Option<RenderId> {
    let mut iter = self.rid.children(self.r_tree);
    let child = iter.next();
    assert!(iter.next().is_none(), "Not only once child.");
    child
  }

  #[cfg(debug_assertions)]
  fn is_child(&self, child: RenderId) -> bool {
    child.ancestors(self.r_tree).find(|r| r == &child).is_some()
  }

  /// Return the render id of the render object this context standard for.
  #[inline]
  pub fn render_id(&self) -> RenderId { self.rid }

  #[inline]
  pub fn render_tree(&self) -> &RenderTree { &self.r_tree }

  /// Return a tuple of [`RenderCtx`]! and  an iterator of children, so you can
  /// avoid the lifetime problem when precess on child.
  pub fn split_children_iter(&mut self) -> (&mut Self, impl Iterator<Item = RenderId> + '_) {
    let rid = self.rid;
    let (ctx, tree) = self.split_r_tree();
    (ctx, rid.children(tree))
  }

  /// Return a tuple of [`RenderCtx`]! and  an reverse iterator of children, so
  /// you can avoid the lifetime problem when precess on child.
  pub fn split_rev_children_iter(&mut self) -> (&mut Self, impl Iterator<Item = RenderId> + '_) {
    let rid = self.rid;
    let (ctx, tree) = self.split_r_tree();
    (ctx, rid.reverse_children(tree))
  }

  pub fn new_ctx(&mut self, other: RenderId) -> RenderCtx {
    RenderCtx {
      r_tree: &mut self.r_tree,
      rid: other,
      layout_store: &mut self.layout_store,
    }
  }

  /// Return render object of this context.
  pub(crate) fn render_obj(&self) -> &dyn RenderObject {
    self
      .rid
      .get(&*self.r_tree)
      .expect("The render object of this context is not exist.")
  }

  /// Perform layout if need, not a public api
  pub(crate) fn perform_layout(&mut self, out_clamp: BoxClamp) -> Size {
    match self.layout_store.layout_info(self.rid) {
      Some(BoxLayout { clamp, rect: Some(rect) }) if &out_clamp == clamp => rect.size,
      _ => {
        let (ctx, r_tree) = self.split_r_tree();
        let size = ctx.rid.get_mut(r_tree).perform_layout(out_clamp, ctx);

        let info = self.layout_store.layout_info_or_default(self.rid);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;
        size
      }
    }
  }

  fn split_r_tree(&mut self) -> (&mut Self, &mut RenderTree) {
    // Safety: split `RenderTree` as two mutable reference is safety, because it's a
    // private inner mutable and promise export only use to access inner object and
    // never modify the tree struct by this reference.
    let r_tree = unsafe { &mut *(self.r_tree as *mut RenderTree) };
    (self, r_tree)
  }
}
