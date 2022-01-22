use crate::prelude::*;
use std::any::{Any, TypeId};
pub mod render_tree;
pub mod update_ctx;
pub use render_tree::{BoxClamp, RenderId};
pub use update_ctx::UpdateCtx;

/// The `Owner` is the render widget which created this object.
pub trait RenderObject: Any {
  /// Do the work of computing the layout for this render object, and return the
  /// size it need.
  ///
  /// In implementing this function, You are responsible for calling every
  /// children's perform_layout across the `RenderCtx`
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size;

  /// Whether the constraints from parent are the only input to detect the
  /// widget size, and child nodes' size not affect its size.
  fn only_sized_by_parent(&self) -> bool;

  /// Paint the render object into `PaintingContext` by itself coordinate
  /// system. Not care about children's paint in this method, framework will
  /// call children's paint individual. And framework guarantee always paint
  /// parent before children.
  fn paint<'a>(&'a self, ctx: &mut PaintingCtx<'a>);

  /// Return a matrix that maps the local logic coordinate system to the local
  /// paint box coordinate system. None-Value means there is not transform
  /// between the coordinate system.
  fn transform(&self) -> Option<Transform> { None }
}

impl<T> RenderWidgetSafety for T
where
  T: RenderWidget,
{
  fn create_render_object(&self) -> Box<dyn RenderObject> {
    let obj = RenderWidget::create_render_object(self);
    Box::new(obj)
  }

  fn update_render_object(&self, object: &mut dyn RenderObject, ctx: &mut UpdateCtx) {
    // SAFETY: framework guarantees that T is the correct type
    let o = unsafe { &mut *(object as *mut dyn RenderObject as *mut T::RO) };
    RenderWidget::update_render_object(self, o, ctx);
  }
}

impl dyn RenderObject {
  /// Returns some reference to the inner value if it is of type `T`, or
  /// `None` if it isn't.
  pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn RenderObject as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      None
    }
  }
}
