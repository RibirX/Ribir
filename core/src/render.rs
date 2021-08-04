pub use render_ctx::*;
pub mod render_ctx;
use crate::prelude::*;
pub use painting_context::*;
use std::any::{Any, TypeId};
mod painting_context;
pub mod render_tree;
pub mod update_ctx;
pub use render_tree::{BoxClamp, RenderId};
pub use update_ctx::UpdateCtx;

/// The `Owner` is the render widget which created this object.
pub trait RenderObject: Sized + Send + Sync + 'static {
  type States: StatePartialEq;
  /// Call by framework when the state of its render widget changed, should not
  /// call this method directly.
  fn update(&mut self, states: Self::States, ctx: &mut UpdateCtx);

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
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);

  /// Return a matrix that maps the local logic coordinate system to the local
  /// paint box coordinate system. None-Value means there is not transform
  /// between the coordinate system.
  fn transform(&self) -> Option<Transform> { None }

  /// Return the states hold in the object.
  fn get_states(&self) -> &Self::States;
}

/// RenderObjectSafety is a object safety trait of RenderObject, never directly
/// implement this trait, just implement [`RenderObject`](RenderObject).
pub trait RenderObjectSafety: Any {
  fn update(&mut self, states: Box<dyn Any>, ctx: &mut UpdateCtx);
  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size;
  fn only_sized_by_parent(&self) -> bool;
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);
  fn transform(&self) -> Option<Transform>;
}

impl<T> RenderWidgetSafety for T
where
  T: RenderWidget,
{
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync> {
    let obj = RenderWidget::create_render_object(self);
    Box::new(obj)
  }

  #[inline]
  fn clone_boxed_states(&self) -> Box<dyn Any> { Box::new(self.clone_states()) }

  #[inline]
  fn get_attrs(&self) -> Option<&Attributes> { RenderWidget::get_attrs(self) }
}

impl<T> RenderObjectSafety for T
where
  T: RenderObject,
{
  #[inline]
  fn update(&mut self, states: Box<dyn Any>, ctx: &mut UpdateCtx) {
    let raw_states = states.downcast_ref::<T::States>().unwrap();
    if !raw_states.eq(self.get_states()) {
      let mut copy = std::mem::MaybeUninit::<T::States>::uninit();
      let copy = unsafe {
        copy
          .as_mut_ptr()
          .copy_from(raw_states as *const T::States, 1);
        copy.assume_init()
      };
      RenderObject::update(self, copy, ctx);
      std::mem::forget(states);
      ctx.mark_needs_layout();
    }
  }

  #[inline]
  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size {
    RenderObject::perform_layout(self, limit, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { RenderObject::only_sized_by_parent(self) }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) { RenderObject::paint(self, ctx); }

  #[inline]
  fn transform(&self) -> Option<Transform> { RenderObject::transform(self) }
}

impl dyn RenderObjectSafety {
  /// Returns some reference to the boxed value if it or its **base widget** is
  /// of type T, or None if it isn't.
  pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn RenderObjectSafety as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      None
    }
  }
}
