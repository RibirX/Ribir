use crate::{prelude::*, render::*};
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
pub use smallvec::{smallvec, SmallVec};
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
pub trait Widget: AsCombination + AsRender + AsAny + StateDetect + 'static {
  /// Return the reference to the attrs that attached to the this widget.
  fn attrs_ref(&self) -> Option<AttrsRef>;

  /// Return the mutable reference to the attrs that attached to the this
  /// widget.
  fn attrs_mut(&mut self) -> Option<AttrsMut>;

  /// Insets the child of a widget by the given padding.
  #[inline]
  fn with_padding(self, edges: EdgeInsets) -> SingleChild<Padding>
  where
    Self: Sized,
  {
    Padding { padding: edges }.with_child(self.box_it())
  }

  /// Create space around the widget
  #[inline]
  fn with_margin(self, edges: EdgeInsets) -> SingleChild<Margin>
  where
    Self: Sized,
  {
    Margin { margin: edges }.with_child(self.box_it())
  }

  /// Sets the background of the widget.
  fn with_background(self, background: FillStyle) -> SingleChild<BoxDecoration>
  where
    Self: Sized,
  {
    // todo: should detect if this widget is a BoxDecoration?
    BoxDecoration::default()
      .with_child(self.box_it())
      .with_background(background)
  }

  /// Set the border of the widget
  fn with_border(self, border: Border) -> SingleChild<BoxDecoration>
  where
    Self: Sized,
  {
    // todo: should detect if this widget is a BoxDecoration?
    BoxDecoration::default()
      .with_child(self.box_it())
      .with_border(border)
  }

  /// Set the radius of the widget.
  fn with_border_radius(self, radius: BorderRadius) -> SingleChild<BoxDecoration>
  where
    Self: Sized,
  {
    // todo: should detect if this widget is a BoxDecoration?
    BoxDecoration::default()
      .with_child(self.box_it())
      .with_border_radius(radius)
  }

  /// Let this widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  fn x_scrollable(self) -> SingleChild<WheelListener<StatefulScrollableX>>
  where
    Self: Sized,
  {
    ScrollableX::x_scroll(0.).with_child(self.box_it())
  }

  /// Let this widget vertical scrollable and the scroll view is as large as
  /// its parent allow.
  fn y_scrollable(self) -> SingleChild<WheelListener<StatefulScrollableY>>
  where
    Self: Sized,
  {
    ScrollableY::y_scroll(0.).with_child(self.box_it())
  }

  /// Let this widget both scrollable in horizontal and vertical, and the scroll
  /// view is as large as its parent allow.
  fn both_scrollable(self) -> SingleChild<WheelListener<StatefulScrollableBoth>>
  where
    Self: Sized,
  {
    ScrollableBoth::both_scroll(Point::zero()).with_child(self.box_it())
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Widget {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget>;
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: Widget + CloneStates + Sized {
  /// The render object type will created.
  type RO: RenderObject<States = Self::States> + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  fn clone_boxed_states(&self) -> Box<dyn Any>;
}

pub trait AsAny {
  fn as_any(&self) -> &dyn Any;

  fn as_any_mut(&mut self) -> &mut dyn Any;
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

pub trait BoxWidget {
  fn box_it(self) -> Box<dyn Widget>;
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

impl<T: Widget + Any> AsAny for T {
  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl<W: Widget> BoxWidget for W {
  #[inline]
  default fn box_it(self) -> Box<dyn Widget> { Box::new(self) }
}

impl<'a> dyn Widget + 'a {
  pub fn key(&self) -> Option<&Key> { self.find_attr() }
}

impl<'a> dyn Widget + 'a {
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
