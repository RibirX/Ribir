use crate::render::render_tree::*;
use crate::render::*;
use canvas::{Canvas, FontInfo, Rect, Text};
use std::pin::Pin;

pub struct RenderCtx<'a> {
  tree: Pin<&'a mut RenderTree>,
  canvas: Pin<&'a mut Canvas>,
  /// the render id of current render object.
  render_obj: RenderId,
}

impl<'a> RenderCtx<'a> {
  #[inline]
  pub(crate) fn new(
    tree: Pin<&'a mut RenderTree>,
    canvas: Pin<&'a mut Canvas>,
    current: RenderId,
  ) -> RenderCtx<'a> {
    RenderCtx {
      tree,
      canvas,
      render_obj: current,
    }
  }

  /// Return the render id of the render object this context provide for.
  #[inline]
  pub fn render_id(&self) -> RenderId { self.render_obj }

  /// Return an iterator of children's `RenderCtx`
  pub fn children<'l>(&'l mut self) -> impl Iterator<Item = RenderCtx<'l>> + 'l {
    // Safety: only split the lifetime for children one by one.
    let (tree_ptr, canvas_ptr) = unsafe {
      let tree_ptr = self.tree.as_mut().get_unchecked_mut() as *mut _;
      let canvas_ptr = self.canvas.as_mut().get_unchecked_mut() as *mut _;
      (tree_ptr, canvas_ptr)
    };

    self
      .render_obj
      .children(&*self.tree)
      .map(move |rid| RenderCtx {
        render_obj: rid,
        tree: unsafe { Pin::new_unchecked(&mut *tree_ptr) },
        canvas: unsafe { Pin::new_unchecked(&mut *canvas_ptr) },
      })
  }

  /// Update the position of the render object should place. Relative to parent.
  pub fn update_position(&mut self, pos: Point) {
    self
      .render_obj
      .layout_box_rect_mut(unsafe { self.tree.as_mut().get_unchecked_mut() })
      .origin = pos;
  }

  /// Return render object of this context.
  pub fn render_obj(&self) -> &dyn RenderObjectSafety {
    self
      .render_obj
      .get(&*self.tree)
      .expect("The render object of this context is not exist.")
  }

  /// Do the work of computing the layout for this render object, and return the
  /// render object box size. Should called from parent.
  pub fn perform_layout(&mut self, clamp: BoxClamp) -> Size {
    if let Some(rect) = self.render_obj.layout_box_rect(&*self.tree) {
      rect.size
    } else {
      // Safety: only split tree from ctx to access the render object instance.
      let tree = unsafe {
        let ptr = self.tree.as_mut().get_unchecked_mut() as *mut RenderTree;
        &mut *ptr
      };
      let size = self
        .render_obj
        .get_mut(tree)
        .expect("must exists")
        .perform_layout(clamp, self);

      *self.render_obj.layout_clamp_mut(tree) = clamp;
      self.render_obj.layout_box_rect_mut(tree).size = size;
      size
    }
  }

  /// mark the render object dirty, will auto diffuse to all the node
  /// affected.
  pub fn mark_layout_dirty(&mut self, mut node_id: RenderId) {
    // if self.is_layout_dirty(node_id) {
    //   return;
    // }
    //   loop {
    //     self.mark_dirty_down(node_id);
    //     let parent_id = node_id.parent(self.tree);
    //     if parent_id.is_none() {
    //       break;
    //     }
    //     let constraints = parent_id
    //       .and_then(|id| id.get(self.tree))
    //       .map(|node| node.get_constraints())
    //       .unwrap();
    //     if !constraints.contains(LayoutConstraints::EFFECTED_BY_CHILDREN) {
    //       break;
    //     }
    //     node_id = parent_id.unwrap();
    //   }
    //   node_id.as_dirty_root(self.tree);
  }

  /// perform layout of all node ignore the cache layout info when force is
  /// true, else perform layout just the dirty layout node
  pub fn layout_tree(&mut self, force: bool) {
    // if force {
    //   self.tree.clean_layout_info();
    //   if let Some(node) = self.tree.root() {
    //     node.as_dirty_root(self.tree);
    //   }
    // }
    // let mut_ptr = self as *mut RenderCtx;
    // for root in self.tree.drain_layout_roots() {
    //   unsafe {
    //     (*mut_ptr).perform_layout(root);
    //   }
    // }
  }

  /// return the layout size. lazy perform layout, if the size has been decided.
  pub fn query_layout_size(&mut self, id: RenderId) -> Size {
    unimplemented!()
    //   let mut size = self.get_layout_size(id);
    //   if size == UNVALID_SIZE {
    //     size = self.perform_layout(id);
    //   }
    //   size
  }

  // mesure test bound
  // todo support custom font
  // pub fn mesure_text(&mut self, text: &str) -> Rect {
  //   let font = FontInfo::default();
  //   self.canvas.mesure_text(&Text {
  //     text,
  //     font_size: 14.0,
  //     font,
  //   })
  // }

  // pub fn collect_children(&mut self, id: RenderId, ids: &mut Vec<RenderId>) {
  //   let mut child = id.first_child(self.tree);
  //   while let Some(child_id) = child {
  //     ids.push(child_id);
  //     child = child_id.next_sibling(self.tree);
  //   }
  // }

  // pub fn get_box_limit(&self, id: RenderId) -> Option<LimitBox> {
  // id.get_box_limit(&self.tree) }

  // pub fn set_box_limit(&mut self, id: RenderId, bound: Option<LimitBox>) {
  //   id.set_box_limit(&mut self.tree, bound);
  // }

  // #[inline]
  // pub fn update_child_pos(&mut self, child: RenderId, pos: Point) {
  //   child.update_position(self.tree, pos);
  // }

  // #[inline]
  // pub fn update_size(&mut self, id: RenderId, size: Size) {
  // id.update_size(self.tree, size); }

  // #[inline]
  // pub fn box_rect(&self, id: RenderId) -> Option<&Rect> {
  // id.box_rect(self.tree) }

  // pub(crate) fn get_layout_size(&self, node_id: RenderId) -> Size {
  //   node_id
  //     .box_rect(&self.tree)
  //     .map(|rect| rect.size)
  //     .unwrap_or(UNVALID_SIZE)
  // }

  // /// get the layout dirty flag.
  // #[inline]
  // pub(crate) fn is_layout_dirty(&self, node_id: RenderId) -> bool {
  //   UNVALID_SIZE == self.get_layout_size(node_id)
  // }

  // fn mark_dirty_down(&mut self, mut id: RenderId) {
  //   if self.is_layout_dirty(id) {
  //     return;
  //   }
  //   id.update_size(self.tree, Size::new(-1.0, -1.0));
  //   let mut ids = vec![];
  //   self.collect_children(id, &mut ids);
  //   while let Some(i) = ids.pop() {
  //     id = i;
  //     if self.mark_constraints_dirty(id, LayoutConstraints::EFFECTED_BY_PARENT)
  // {       self.collect_children(id, &mut ids);
  //     }
  //   }
  // }

  // fn mark_constraints_dirty(&mut self, id: RenderId, target: LayoutConstraints)
  // -> bool {   let constraints = id
  //     .get(self.tree)
  //     .map(|node| node.get_constraints())
  //     .unwrap();
  //   if constraints.intersects(target) {
  //     id.update_size(self.tree, Size::new(-1.0, -1.0));
  //     true
  //   } else {
  //     false
  //   }
  // }
}
