use crate::widget::Key;
use std::fmt::Debug;
use std::raw::TraitObject;

/// RenderWidget provide configuration for render object which provide actual
/// rendering and paint for the application.
pub trait RenderWidget: Debug + Sized {
  /// The render object type will created.
  type RO: RenderObject<Owner = Self>;

  /// `Key` help `Holiday` to track if two widget is a same widget in two frame.
  /// You should not override this method, use [`KeyDetect`](key::KeyDetect) if
  /// you want give a key to your widget.
  fn key(&self) -> Option<&Key> { None }

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;
}
pub trait RenderObject: Debug + Sized {
  /// The render widget which created this object.
  type Owner: RenderWidget<RO = Self>;

  /// Call by framework when its owner render widget `owner_widget`
  /// changed, should not call this method directly.
  fn update(&mut self, owner_widget: &Self::Owner);
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety: Debug {
  fn key(&self) -> Option<&Key> { None }
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
}

/// RenderObjectSafety is a object safety trait of RenderObject, never directly
/// implement this trait, just implement [`RenderObject`](RenderObject).
pub trait RenderObjectSafety: Debug {
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety);
}

fn downcast_widget<T: RenderWidget>(obj: &dyn RenderWidgetSafety) -> &T {
  unsafe {
    let trait_obj: TraitObject = std::mem::transmute(obj);
    &*(trait_obj.data as *const T)
  }
}

fn downcast_widget_mut<T: RenderWidget>(
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
  T::RO: RenderObjectSafety + Send + Sync + 'static,
{
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync> {
    Box::new(RenderWidget::create_render_object(self))
  }
}

impl<T> RenderObjectSafety for T
where
  T: RenderObject + 'static,
{
  fn update(&mut self, owner_widget: &dyn RenderWidgetSafety) {
    RenderObject::update(self, downcast_widget(owner_widget))
  }
}
