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
