use crate::render::render_tree::RenderId;

mod box_constraint;
pub use box_constraint::*;
pub use render_ctx::*;
pub mod render_ctx;
use crate::{prelude::Point, prelude::Size, widget::Key};
pub use painting_context::PaintingContext;
use std::fmt::Debug;
use std::raw::TraitObject;
pub mod painting_context;
pub mod render_tree;

bitflags! {
    pub struct LayoutConstraints: u8 {
        const DECIDED_BY_SELF = 0;
        const EFFECTED_BY_PARENT = 1;
        const EFFECTED_BY_CHILDREN = 2;
    }
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering and paint for the application.
pub trait RenderWidget: Debug + Sized {
  /// The render object type will created.
  type RO: RenderObject<Owner = Self> + Send + Sync + 'static;

  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use
  /// [`KeyDetect`](crate::widget::key::KeyDetect) if you want give a key to
  /// your widget.
  fn key(&self) -> Option<&Key> { None }

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

  // trig the process of layout
  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) -> Size;

  // get layout constraints type;
  fn get_constraints(&self) -> LayoutConstraints;

  // set layout bound limit
  fn set_box_limit(&mut self, bound: Option<BoxLimit>);

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
  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) -> Size;
  fn get_constraints(&self) -> LayoutConstraints;
  /// set layout limitation to the render object.
  fn set_box_limit(&mut self, bound: Option<BoxLimit>);
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>);
}

fn downcast_widget<T: RenderWidget>(obj: &dyn RenderWidgetSafety) -> &T {
  unsafe {
    let trait_obj: TraitObject = std::mem::transmute(obj);
    &*(trait_obj.data as *const T)
  }
}

#[allow(dead_code)]
fn downcast_widget_mut<T: RenderWidget>(obj: &mut dyn RenderWidgetSafety) -> &mut T {
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
  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) -> Size {
    RenderObject::perform_layout(self, id, ctx)
  }

  #[inline]
  fn get_constraints(&self) -> LayoutConstraints { RenderObject::get_constraints(self) }
  #[inline]
  fn set_box_limit(&mut self, bound: Option<BoxLimit>) { RenderObject::set_box_limit(self, bound) }
  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) { RenderObject::paint(self, ctx); }
}
