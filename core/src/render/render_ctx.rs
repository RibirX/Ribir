use crate::render::render_tree::*;
use crate::render::*;
use canvas::{Canvas, FontInfo, Rect, Text};
use std::pin::Pin;

/// A place to compute the render object's layout. Rather than holding children
/// directly, `RenderObject` perform layout across `RenderCtx`. `RenderCtx`
/// provide method to perform layout and also provides methods to access the
/// `RenderCtx` of the children.
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

  /// Return the render id of the render object this context standard for.
  #[inline]
  pub fn render_id(&self) -> RenderId { self.render_obj }

  /// Return an iterator of children's `RenderCtx`
  pub fn children(&mut self) -> impl Iterator<Item = RenderCtx> + '_ {
    // Safety: only split the lifetime for children one by one, and `RenderCtx` will
    // not provide method to change the render tree.
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

  /// Returns an iterator of RenderId of this RenderObject’s children, in
  /// reverse order.
  pub fn reverse_children(&mut self) -> impl Iterator<Item = RenderCtx> + '_ {
    // Safety: only split the lifetime for children one by one.
    let (tree_ptr, canvas_ptr) = unsafe {
      let tree_ptr = self.tree.as_mut().get_unchecked_mut() as *mut _;
      let canvas_ptr = self.canvas.as_mut().get_unchecked_mut() as *mut _;
      (tree_ptr, canvas_ptr)
    };

    self
      .render_obj
      .reverse_children(&*self.tree)
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

  /// Update the size of the render object should place. Use this method to
  /// directly change the size of a render object, in most cast you needn't call
  /// this method, use  clamp to limit the child size is enough. Use this method
  /// only it you know what you are doing.
  pub fn update_size(&mut self, size: Size) {
    self
      .render_obj
      .layout_box_rect_mut(unsafe { self.tree.as_mut().get_unchecked_mut() })
      .size = size;
  }

  /// Return the boxed rect of the RenderObject already placed.
  pub fn box_rect(&self) -> Option<Rect> { self.render_obj.layout_box_rect(&*self.tree) }

  /// Return render object of this context.
  pub fn render_obj(&self) -> &(dyn RenderObjectSafety + 'static) {
    self
      .render_obj
      .get(&*self.tree)
      .expect("The render object of this context is not exist.")
  }

  /// Do the work of computing the layout for this render object, and return the
  /// render object box size. Should called from parent.
  pub fn perform_layout(&mut self, clamp: BoxClamp) -> Size {
    self
      .render_obj
      .perform_layout(clamp, self.canvas.as_mut(), self.tree.as_mut())
  }

  // mesure test bound
  // todo support custom font
  pub fn mesure_text(&mut self, text: &str) -> Rect {
    let font = FontInfo::default();
    self.canvas.mesure_text(&Text {
      text,
      font_size: 14.0,
      font,
    })
  }
}
