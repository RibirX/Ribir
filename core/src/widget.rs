#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  collections::HashMap,
};
pub mod build_ctx;
pub mod key;
pub mod layout;
pub use layout::*;
pub mod stateful;
pub mod text;
mod theme;
pub use theme::*;
pub mod widget_tree;
pub mod window;
pub use build_ctx::BuildCtx;
pub use key::Key;
pub use stateful::*;
pub use text::Text;
pub mod events;
pub use events::*;
mod cursor;
pub use cursor::Cursor;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
pub use padding::*;
mod box_decoration;
pub use box_decoration::*;
mod attr;
pub use attr::*;
mod checkbox;
pub use checkbox::*;
mod scrollable;
pub use scrollable::*;

/// A widget represented by other widget compose.
pub trait CombinationWidget: AttrsAccess + 'static {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget;
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: AttrsAccess + 'static {
  /// The render object type will created.
  type RO: RenderObject + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;

  /// update the render object when the render widget is changed, and if the
  /// change effect to the layout remember to call `ctx.mark_needs_layout()`
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx);
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety: AttrsAccess {
  fn create_render_object(&self) -> Box<dyn RenderObject + Send + Sync>;

  fn update_render_object(&self, object: &mut dyn RenderObject, ctx: &mut UpdateCtx);
}

pub type BoxedSingleChild = Box<SingleChild<Box<dyn RenderWidgetSafety>>>;
pub type BoxedMultiChild = MultiChild<Box<dyn RenderWidgetSafety>>;
pub enum BoxedWidget {
  Combination(Box<dyn CombinationWidget>),
  Render(Box<dyn RenderWidgetSafety>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
}

impl AttrsAccess for BoxedWidget {
  fn get_attrs(&self) -> Option<&Attributes> {
    match self {
      BoxedWidget::Combination(c) => c.get_attrs(),
      BoxedWidget::Render(r) => r.get_attrs(),
      BoxedWidget::SingleChild(s) => s.get_attrs(),
      BoxedWidget::MultiChild(m) => m.get_attrs(),
    }
  }

  fn get_attrs_mut(&mut self) -> Option<&mut Attributes> {
    match self {
      BoxedWidget::Combination(c) => c.get_attrs_mut(),
      BoxedWidget::Render(r) => r.get_attrs_mut(),
      BoxedWidget::SingleChild(s) => s.get_attrs_mut(),
      BoxedWidget::MultiChild(m) => m.get_attrs_mut(),
    }
  }
}

// Widget & BoxWidget default implementation

pub struct CombinationMarker;
pub struct RenderMarker;
pub trait BoxWidget<Marker> {
  fn box_it(self) -> BoxedWidget;
}

impl<T: CombinationWidget> BoxWidget<()> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget { BoxedWidget::Combination(Box::new(self)) }
}

impl<T: RenderWidget> BoxWidget<RenderMarker> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget { BoxedWidget::Render(Box::new(self)) }
}

impl<S> BoxWidget<()> for SingleChild<S>
where
  S: RenderWidget,
{
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    let boxed = Box::new(SingleChild { widget, child: self.child });
    BoxedWidget::SingleChild(boxed)
  }
}

impl<M: MultiChildWidget> BoxWidget<()> for MultiChild<M> {
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderWidgetSafety> = Box::new(self.widget);
    BoxedWidget::MultiChild(MultiChild { widget, children: self.children })
  }
}

impl BoxWidget<()> for BoxedWidget {
  #[inline]
  fn box_it(self) -> BoxedWidget { self }
}
