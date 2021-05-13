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
pub trait Widget: AsCombination + AsRender + AsAny + 'static {
  /// Find an attr of this widget. If it have the `A` type attr, return the
  /// reference.
  fn find_attr<A: Any>(&self) -> Option<&A>;

  /// Find an attr of this widget. If it have the `A` type attr, return the
  /// mutable reference.
  fn find_attr_mut<A: Any>(&mut self) -> Option<&mut A>;

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    BoxWidget { widget: Box::new(self) }
  }

  /// Insets the child of a widget by the given padding.
  #[inline]
  fn with_padding(self, edges: EdgeInsets) -> Padding
  where
    Self: Sized,
  {
    Padding { padding: edges, child: self.box_it() }
  }

  /// Create space around the widget
  #[inline]
  fn with_margin(self, edges: EdgeInsets) -> Margin
  where
    Self: Sized,
  {
    Margin { margin: edges, child: self.box_it() }
  }

  /// Sets the background of the widget.
  fn with_background(self, background: FillStyle) -> BoxDecoration
  where
    Self: Sized,
  {
    BoxDecoration::new(self.box_it()).with_background(background)
  }

  /// Set the border of the widget
  fn with_border(self, border: Border) -> BoxDecoration
  where
    Self: Sized,
  {
    BoxDecoration::new(self.box_it()).with_border(border)
  }

  /// Set the radius of the widget.
  fn with_border_radius(self, radius: BorderRadius) -> BoxDecoration
  where
    Self: Sized,
  {
    BoxDecoration::new(self.box_it()).with_border_radius(radius)
  }

  /// Let this widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  fn x_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<RcWidget<ScrollableX>>
  where
    Self: Sized,
  {
    ScrollableX::new(self.box_it(), 0., ctx)
  }

  /// Let this widget vertical scrollable and the scroll view is as large as
  /// its parent allow.
  fn y_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<RcWidget<ScrollableY>>
  where
    Self: Sized,
  {
    ScrollableY::new(self.box_it(), 0., ctx)
  }

  /// Let this widget both scrollable in horizontal and vertical, and the scroll
  /// view is as large as its parent allow.
  fn both_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<RcWidget<ScrollableBoth>>
  where
    Self: Sized,
  {
    ScrollableBoth::new(self.box_it(), Point::zero(), ctx)
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Widget {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget;

  fn state_ref_cell(&self, ctx: &mut BuildCtx) -> StateRefCell<Self>
  where
    Self: Sized,
  {
    unimplemented!();
    // if let Some(stateful) = ctx.widget().widget.find_attr::<StatefulAttr>() {
    //   let widget =
    // ctx.widget().as_any().downcast_ref::<RcWidget<Self>>().expect("stateful
    // attr not hold a");   unsafe { stateful.attr.ref_cell() }
    // } else {
    //   let attr = ctx.state_attr();

    //   let widget = std::mem::replace(ctx.widget_mut(),
    // PhantomWidget.box_it());

    //   let stateful = widget.attach_attr(attr).box_it();
    //   let _ = std::mem::replace(ctx.widget_mut(), stateful);
    //   ref_cell
    // }
  }
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: Widget + Sized {
  /// The render object type will created.
  type RO: RenderObject<Owner = Self> + Send + Sync + 'static;

  /// Creates an instance of the RenderObject that this RenderWidget
  /// represents, using the configuration described by this RenderWidget
  fn create_render_object(&self) -> Self::RO;

  /// Called by framework to take children from this widget, return some-value
  /// to if it has child, else return None. This method will only be called
  /// once. Should never directly call it.
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>>;
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>>;
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
  default fn as_any(&self) -> &dyn Any { self }

  #[inline]
  default fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Todo: Remove BoxWidget after support specialization Box<dyn Widget>
pub struct BoxWidget {
  pub(crate) widget: Box<dyn Widget>,
}

impl AsAny for BoxWidget {
  #[inline]
  fn as_any(&self) -> &dyn Any { self.widget.as_any() }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self.widget.as_any_mut() }
}

impl<'a> dyn Widget + 'a {
  pub fn key(&self) -> Option<&Key> { self.find_attr() }
}

impl Widget for BoxWidget {
  #[inline]
  fn find_attr<A: Any>(&self) -> Option<&A> { self.widget.find_attr() }

  #[inline]
  fn find_attr_mut<A: Any>(&mut self) -> Option<&mut A> { self.widget.find_attr_mut() }

  #[inline]
  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    self
  }
}

impl BoxWidget {
  #[inline]
  pub fn key(&self) -> Option<&Key> { self.widget.key() }
}

proxy_impl_as_trait!(BoxWidget, widget);

impl From<Box<dyn Widget>> for BoxWidget {
  #[inline]
  fn from(widget: Box<dyn Widget>) -> Self { Self { widget } }
}

pub(crate) macro proxy_impl_as_trait(
  $name: ty,
  $proxy_member: tt) {
  impl AsCombination for $name {
    #[inline]
    fn as_combination(&self) -> Option<&dyn CombinationWidget> {
      self.$proxy_member.as_combination()
    }

    #[inline]
    fn as_combination_mut(&mut self) -> Option<&mut dyn CombinationWidget> {
      self.$proxy_member.as_combination_mut()
    }
  }

  impl AsRender for $name {
    #[inline]
    fn as_render(&self) -> Option<&dyn RenderWidgetSafety> { self.$proxy_member.as_render() }

    #[inline]
    fn as_render_mut(&mut self) -> Option<&mut dyn RenderWidgetSafety> {
      self.$proxy_member.as_render_mut()
    }
  }

  // AsAny should not be proxy.
}
