pub use render_ctx::*;
pub mod render_ctx;
use crate::prelude::*;
pub use painting_context::PaintingContext;
use std::fmt::Debug;
pub mod painting_context;
pub mod render_tree;
pub use render_tree::{BoxClamp, RenderId};

/// RenderWidget provide configuration for render object which provide actual
/// rendering and paint for the application.
pub trait RenderWidget: Debug + Sized {
  /// The render object type will created.
  type RO: RenderObject<Owner = Self> + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;
}

/// The `Owner` is the render widget which created this object.
pub trait RenderObject: Debug + Sized + Send + Sync + 'static {
  type Owner: RenderWidget<RO = Self>;
  /// Call by framework when its owner `owner_widget` changed, should not call
  /// this method directly.
  fn update(&mut self, owner_widget: &Self::Owner);

  /// Do the work of computing the layout for this render object, and return the
  /// render object size after layout.
  ///
  /// In implementing this function, you must call layout on each of your
  /// children
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size;

  /// Whether the constraints from parent are the only input to detect the
  /// widget size, and child nodes' size have no effect it.
  fn only_sized_by_parent(&self) -> bool;

  /// Paint the render object into `PaintingContext` by itself coordinate
  /// system. Not care about children's paint in this method, framework will
  /// call children's paint individual. And framework guarantee always paint
  /// parent before children.
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety: Debug {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  /// This method is provide to SubTrait upcast to a `RenderWidgetSafety`
  /// reference.
  fn as_render(&self) -> &dyn RenderWidgetSafety;
  /// This method is provide to SubTrait upcast to a mutation
  /// `RenderWidgetSafety` reference.
  fn as_render_mut(&mut self) -> &mut dyn RenderWidgetSafety;
}

/// RenderObjectSafety is a object safety trait of RenderObject, never directly
/// implement this trait, just implement [`RenderObject`](RenderObject).
pub trait RenderObjectSafety: Debug {
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety);
  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size;
  fn only_sized_by_parent(&self) -> bool;
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);
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
  fn as_render(&self) -> &dyn RenderWidgetSafety { self }

  #[inline]
  fn as_render_mut(&mut self) -> &mut dyn RenderWidgetSafety { self }
}

impl<T> RenderObjectSafety for T
where
  T: RenderObject,
{
  #[inline]
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety) {
    RenderObject::update(self, downcast_widget(owner_widget))
  }

  #[inline]
  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size {
    RenderObject::perform_layout(self, limit, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { RenderObject::only_sized_by_parent(self) }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) { RenderObject::paint(self, ctx); }
}
