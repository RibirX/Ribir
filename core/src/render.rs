use crate::render::render_tree::RenderId;
use crate::widget::Key;

use std::fmt::Debug;
use std::raw::TraitObject;
mod base;
mod box_constraint;
pub use base::*;
pub use box_constraint::*;
pub use render_ctx::*;
pub mod render_ctx;
pub mod render_tree;

/// RenderWidget provide configuration for render object which provide actual
/// rendering and paint for the application.
pub trait RenderWidget: Debug + Sized {
  /// The render object type will created.
  type RO: RenderObject<Self> + Send + Sync + 'static;

  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use
  /// [`KeyDetect`](crate::widget::key::KeyDetect) if you want give a key to
  /// your widget.
  fn key(&self) -> Option<&Key> { None }

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;
}

/// The `Owner` is the render widget which created this object. And it's should
/// be a associated type instead of generic type after rust support GAT.
pub trait RenderObject<Owner: RenderWidget<RO = Self>>:
  Debug + Sized + Send + Sync + 'static
{
  /// Call by framework when its owner `owner_widget` changed, should not call
  /// this method directly.
  fn update(&mut self, owner_widget: &Owner);

  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx);

  fn get_size(&self) -> Option<Size>;
  fn get_constraints(&self) -> LayoutConstraints;
  fn set_box_bound(&mut self, bound: Option<BoxBound>);
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
  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx);
  fn get_size(&self) -> Option<Size>;
  fn get_constraints(&self) -> LayoutConstraints;
  fn set_box_bound(&mut self, bound: Option<BoxBound>);
}

pub(crate) fn downcast_widget<T: RenderWidget>(
  obj: &dyn RenderWidgetSafety,
) -> &T {
  unsafe {
    let trait_obj: TraitObject = std::mem::transmute(obj);
    &*(trait_obj.data as *const T)
  }
}

pub(crate) fn downcast_widget_mut<T: RenderWidget>(
  obj: &mut dyn RenderWidgetSafety,
) -> &mut T {
  unsafe {
    let trait_obj: TraitObject = std::mem::transmute(obj);
    &mut *(trait_obj.data as *mut T)
  }
}

impl<T> RenderWidgetSafety for T
where
  T: RenderWidget,
{
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync> {
    let r_box = RenderObjectBox {
      render: RenderWidget::create_render_object(self),
      _marker: PhantomData,
    };
    r_box.to_safety()
  }

  #[inline]
  fn as_render(&self) -> &dyn RenderWidgetSafety { self }

  #[inline]
  fn as_render_mut(&mut self) -> &mut dyn RenderWidgetSafety { self }
}

use std::marker::PhantomData;
/// Because `Owner` is a generic type of RenderObject trait, so we can't auto
/// implement  RenderObjectSafety for type which implemented
/// `RenderWidget<Owner>`, because unconstrained problem and associated item
/// lifetime not support. So provide RenderObjectBox to implement
/// RenderObjectSafety. It's not a elegant way and looks too tricky. After GAT
/// is supported we let `Owner` as an associated item instead of generic type on
/// `RenderObject`, and directly impl RenderObjectSafety like
/// RenderWidgetSafety.
pub struct RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
  render: R,
  _marker: PhantomData<*const W>,
}

unsafe impl<W, R> Send for RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
}

unsafe impl<W, R> Sync for RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
}

use std::fmt::{Formatter, Result};
impl<W, R> Debug for RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> Result { self.render.fmt(f) }
}

impl<W, R> RenderObjectSafety for RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety) {
    RenderObject::update(&mut self.render, downcast_widget(owner_widget))
  }

  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) {
    RenderObject::perform_layout(&mut self.render, id, ctx)
  }

  fn get_size(&self) -> Option<Size> { RenderObject::get_size(&self.render) }
  fn get_constraints(&self) -> LayoutConstraints {
    RenderObject::get_constraints(&self.render)
  }
  fn set_box_bound(&mut self, bound: Option<BoxBound>) {
    RenderObject::set_box_bound(&mut self.render, bound)
  }
}

impl<W, R> RenderObjectBox<W, R>
where
  W: RenderWidget<RO = R>,
  R: RenderObject<W>,
{
  fn to_safety(self) -> Box<dyn RenderObjectSafety + Send + Sync + 'static> {
    let safety: Box<dyn RenderObjectSafety + Send + Sync> = Box::new(self);
    // unsafe introduce: `W` just use to constraint type, and never access it.
    // And `R` bounds with RenderObject should always `static` lifetime.
    // This will be removed after rust GAT supported.
    unsafe { std::mem::transmute(safety) }
  }
}
