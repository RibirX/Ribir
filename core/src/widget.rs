use crate::render::*;
pub use std::any::Any;
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
pub use key::{Key, KeyDetect};
pub use stateful::*;
pub use text::Text;
pub mod events;
pub use events::*;
mod phantom;
pub use phantom::PhantomWidget;
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

/// The common behavior of widgets, also support to dynamic cast to special
/// widget. In most of cases, needn't implement `Widget` trait directly, and
/// implement `CombinationWidget`, `RenderWidget` instead of
pub trait Widget: AsCombination + AsRender + Any + StateDetect + 'static {
  /// Return the reference to the attrs that attached to the this widget.
  #[inline]
  fn attrs_ref(&self) -> Option<AttrsRef> { None }

  /// Return the mutable reference to the attrs that attached to the this
  /// widget.
  #[inline]
  fn attrs_mut(&mut self) -> Option<AttrsMut> { None }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Widget + AsWidget {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget;

  fn box_it(self) -> BoxedWidget
  where
    Self: Sized,
  {
    BoxedWidget::Combination(Box::new(self))
  }
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: Widget + CloneStates {
  /// The render object type will created.
  type RO: RenderObject<States = Self::States> + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;

  fn box_it(self) -> BoxedWidget
  where
    Self: Sized,
  {
    BoxedWidget::Render(Box::new(self))
  }
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety: Widget + AsWidget {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  fn clone_boxed_states(&self) -> Box<dyn Any>;
}

pub trait AsCombination {
  /// return some-value  of `CombinationWidget` reference if this widget is a
  /// combination widget.`
  fn as_combination(&self) -> Option<&dyn CombinationWidget>;

  /// return some-value of `CombinationWidget` mutable reference if this widget
  /// is a combination widget.
  fn as_combination_mut(&mut self) -> Option<&mut dyn CombinationWidget>;
}

pub trait AsRender {
  /// return some-value of `RenderWidgetSafety` reference if this widget
  /// is a render widget.
  fn as_render(&self) -> Option<&dyn RenderWidgetSafety>;

  /// return some-value of `RenderWidgetSafety` mutable reference if this widget
  /// is a render widget.
  fn as_render_mut(&mut self) -> Option<&mut dyn RenderWidgetSafety>;
}

pub enum BoxedWidget {
  Combination(Box<dyn CombinationWidget>),
  Render(Box<dyn RenderWidgetSafety>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
}

impl<T: Widget> AsCombination for T {
  #[inline]
  default fn as_combination(&self) -> Option<&dyn CombinationWidget> { None }

  #[inline]
  default fn as_combination_mut(&mut self) -> Option<&mut dyn CombinationWidget> { None }
}

impl<T: CombinationWidget> AsCombination for T {
  #[inline]
  fn as_combination(&self) -> Option<&dyn CombinationWidget> { Some(self) }

  #[inline]
  fn as_combination_mut(&mut self) -> Option<&mut dyn CombinationWidget> { Some(self) }
}

impl<T: Widget> AsRender for T {
  #[inline]
  default fn as_render(&self) -> Option<&dyn RenderWidgetSafety> { None }

  #[inline]
  default fn as_render_mut(&mut self) -> Option<&mut dyn RenderWidgetSafety> { None }
}

impl<T: RenderWidget> AsRender for T {
  #[inline]
  fn as_render(&self) -> Option<&dyn RenderWidgetSafety> { Some(self) }

  #[inline]
  fn as_render_mut(&mut self) -> Option<&mut dyn RenderWidgetSafety> { Some(self) }
}

impl<'a> dyn Widget + 'a {
  /// Return the `Key` attribute of the widget.
  pub fn key(&self) -> Option<&Key> { self.find_attr() }

  /// Find an attr of this widget. If it have the `A` type attr, return the
  /// reference.
  pub fn find_attr<A: Any>(&self) -> Option<&A> {
    self.attrs_ref().and_then(|attrs| attrs.find_attr())
  }

  /// Find an attr of this widget. If it have the `A` type attr, return the
  /// mutable reference.
  pub fn find_attr_mut<A: Any>(&mut self) -> Option<&mut A> {
    self.attrs_mut().and_then(|attrs| attrs.find_attr_mut())
  }
}

pub trait AsWidget {
  fn as_widget(&self) -> &dyn Widget;
}

impl<W: Widget> AsWidget for W {
  #[inline]
  fn as_widget(&self) -> &dyn Widget { self }
}

impl BoxedWidget {
  pub fn key(&self) -> Option<&Key> {
    let w = match self {
      BoxedWidget::Combination(c) => c.as_widget(),
      BoxedWidget::Render(r) => r.as_widget(),
      BoxedWidget::SingleChild(s) => s.as_widget(),
      BoxedWidget::MultiChild(m) => m.as_widget(),
    };
    w.key()
  }
}
