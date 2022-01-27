#[doc(hidden)]
pub use std::any::{Any, TypeId};
pub mod key;
pub mod layout;
pub use layout::*;
pub mod stateful;
pub mod text;
mod theme;
pub use theme::*;
pub(crate) mod widget_tree;
pub mod window;
pub use crate::widget::text::Text;
pub use key::Key;
pub use stateful::*;
mod cursor;
pub use cursor::Cursor;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
pub use padding::*;
mod box_decoration;
pub use box_decoration::*;
pub mod attr;
pub use attr::*;
mod checkbox;
pub use checkbox::*;
mod scrollable;
pub use scrollable::*;
mod path;
pub use path::*;

/// A widget represented by other widget compose.
pub trait CombinationWidget {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: BuildCtx<Self>) -> BoxedWidget
  where
    Self: Sized;
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget {
  /// The render object type will created.
  type RO: RenderObject;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;

  /// update the render object when the render widget is changed, and if the
  /// change effect to the layout remember to call `ctx.mark_needs_layout()`
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx);
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety {
  fn create_render_object(&self) -> Box<dyn RenderObject>;

  fn update_render_object(&self, object: &mut dyn RenderObject, ctx: &mut UpdateCtx);
}

pub struct BoxedWidget(BoxedWidgetInner);

#[macro_export]
macro_rules! mark_layout_assign {
  ($left: expr, $right: expr, $ctx: ident) => {
    if &$left != &$right {
      $left = $right.clone();
      $ctx.mark_needs_layout();
    }
  };
}

#[marker]
pub trait Widget {}
impl<W: CombinationWidget> Widget for W {}
impl<W: RenderWidget> Widget for W {}

pub trait IntoRender {
  type R: RenderWidget;
  fn into_render(self) -> Self::R;
}

pub trait IntoCombination {
  type C: CombinationWidget;
  fn into_combination(self) -> Self::C;
}

impl<W: RenderWidget> IntoRender for W {
  type R = W;
  #[inline]
  fn into_render(self) -> Self::R { self }
}

impl<W: CombinationWidget> IntoCombination for W {
  type C = W;
  #[inline]
  fn into_combination(self) -> Self::C { self }
}

pub(crate) type BoxedSingleChild = Box<SingleChild<Box<dyn RenderNode>>>;
pub(crate) type BoxedMultiChild = MultiChild<Box<dyn RenderNode>>;
pub(crate) trait CombinationNode: AsAttrs {
  fn build(&self, self_id: WidgetId, ctx: &Context) -> BoxedWidget;
}
pub(crate) trait RenderNode: RenderWidgetSafety + AsAttrs {}

impl<W: CombinationWidget + AsAttrs> CombinationNode for W {
  fn build(&self, self_id: WidgetId, ctx: &Context) -> BoxedWidget {
    let ctx = BuildCtx::new(ctx, self_id);
    self.build(ctx)
  }
}
impl<W: RenderWidget + AsAttrs> RenderNode for W {}

pub(crate) enum BoxedWidgetInner {
  Combination(Box<dyn CombinationNode>),
  Render(Box<dyn RenderNode>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
}

// Widget & BoxWidget default implementation

pub struct CombinationMarker;
pub struct RenderMarker;
pub trait BoxWidget<Marker> {
  fn box_it(self) -> BoxedWidget;
}

impl<T: IntoCombination + 'static> BoxWidget<CombinationMarker> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    BoxedWidget(BoxedWidgetInner::Combination(Box::new(
      self.into_combination(),
    )))
  }
}

impl<T: IntoRender + 'static> BoxWidget<RenderMarker> for T {
  #[inline]
  fn box_it(self) -> BoxedWidget {
    BoxedWidget(BoxedWidgetInner::Render(Box::new(self.into_render())))
  }
}

impl<S: SingleChildWidget + 'static> BoxWidget<RenderMarker> for SingleChild<S> {
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget.into_render());
    let boxed = Box::new(SingleChild { widget, child: self.child });
    BoxedWidget(BoxedWidgetInner::SingleChild(boxed))
  }
}

impl<M: MultiChildWidget + 'static> BoxWidget<RenderMarker> for MultiChild<M> {
  fn box_it(self) -> BoxedWidget {
    let widget: Box<dyn RenderNode> = Box::new(self.widget.into_render());
    let inner = BoxedWidgetInner::MultiChild(MultiChild { widget, children: self.children });
    BoxedWidget(inner)
  }
}

impl BoxWidget<()> for BoxedWidget {
  #[inline]
  fn box_it(self) -> BoxedWidget { self }
}
