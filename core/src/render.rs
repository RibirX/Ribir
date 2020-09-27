pub use render_ctx::*;
pub mod render_ctx;
use crate::prelude::*;
pub use painting_context::*;
use std::{
  any::{Any, TypeId},
  fmt::Debug,
};
mod painting_context;
pub mod render_tree;
pub mod update_ctx;
pub use render_tree::{BoxClamp, RenderId};
pub use update_ctx::UpdateCtx;

/// The `Owner` is the render widget which created this object.
pub trait RenderObject: Debug + Sized + Send + Sync + 'static {
  type Owner: RenderWidget<RO = Self>;
  /// Call by framework when its owner `owner_widget` changed, should not call
  /// this method directly.
  fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx);

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

  /// Returns a matrix that maps the local logic coordinate system to the local
  /// paint box coordinate system. None-Value means there is not transform
  /// between the coordinate system.
  fn transform(&self) -> Option<Transform> { None }
}

/// RenderObjectSafety is a object safety trait of RenderObject, never directly
/// implement this trait, just implement [`RenderObject`](RenderObject).
pub trait RenderObjectSafety: Debug + Any {
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety, ctx: &mut UpdateCtx);
  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size;
  fn only_sized_by_parent(&self) -> bool;
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);
  fn transform(&self) -> Option<Transform>;
}

fn downcast_widget<T: RenderWidget>(obj: &dyn RenderWidgetSafety) -> &T {
  let ptr = obj as *const dyn RenderWidgetSafety as *const T;
  // SAFETY: in this mod, we know obj must be type `T`.
  unsafe { &*ptr }
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
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    RenderWidget::take_children(self)
  }
}

impl<T> RenderObjectSafety for T
where
  T: RenderObject,
{
  #[inline]
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety, ctx: &mut UpdateCtx) {
    RenderObject::update(self, downcast_widget(owner_widget), ctx)
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
