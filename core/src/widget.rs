use crate::{prelude::*, render::*};
use std::{any::Any, fmt::Debug};
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
pub use stateful::{StateChange, StateRefCell, Stateful};
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
use widget::stateful::StatefulAttr;
mod scrollable;
pub use scrollable::*;

/// The common behavior of widgets, also support to dynamic cast to special
/// widget. In most of cases, needn't implement `Widget` trait directly, and
/// implement `CombinationWidget`, `RenderWidget` instead of
pub trait Widget: AsCombination + AsRender + AsAny + AsAttr + Debug + 'static {
  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    BoxWidget {
      widget: Box::new(self),
    }
  }

  /// Insets the child of a widget by the given padding.
  #[inline]
  fn with_padding(self, edges: EdgeInsets) -> Padding
  where
    Self: Sized,
  {
    Padding {
      padding: edges,
      child: self.box_it(),
    }
  }

  /// Create space around the widget
  #[inline]
  fn with_margin(self, edges: EdgeInsets) -> Margin
  where
    Self: Sized,
  {
    Margin {
      margin: edges,
      child: self.box_it(),
    }
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
  fn x_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<ScrollableX>
  where
    Self: Sized,
  {
    ScrollableX::new(self.box_it(), 0., ctx)
  }

  /// Let this widget vertical scrollable and the scroll view is as large as
  /// its parent allow.
  fn y_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<ScrollableY>
  where
    Self: Sized,
  {
    ScrollableY::new(self.box_it(), 0., ctx)
  }

  /// Let this widget both scrollable in horizontal and vertical, and the scroll
  /// view is as large as its parent allow.
  fn both_scrollable(self, ctx: &mut BuildCtx) -> WheelListener<ScrollableBoth>
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
    if let Some(stateful) = ctx.widget().downcast_attr_widget::<StatefulAttr>() {
      unsafe { stateful.attr.ref_cell() }
    } else {
      let attr = ctx.state_attr();
      let ref_cell = unsafe { attr.ref_cell() };

      let widget = std::mem::replace(ctx.widget_mut(), PhantomWidget.box_it());
      let stateful = widget.attach_attr(attr).box_it();
      let _ = std::mem::replace(ctx.widget_mut(), stateful);
      ref_cell
    }
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
pub trait RenderWidgetSafety: Debug {
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

pub trait AsAttr {
  /// return the some-value of `WidgetAttr` reference if the widget attached
  /// attr.
  fn as_attr(&self) -> Option<&dyn Attribute>;

  /// like `as_attr`, but return mutable reference.
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute>;
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

impl<T: Widget> AsAttr for T {
  default fn as_attr(&self) -> Option<&dyn Attribute> { None }

  default fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { None }
}

// Todo: Remove BoxWidget after support specialization Box<dyn Widget>
pub struct BoxWidget {
  pub(crate) widget: Box<dyn Widget>,
}

impl std::fmt::Debug for BoxWidget {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.widget.fmt(f) }
}

impl AsAny for BoxWidget {
  #[inline]
  fn as_any(&self) -> &dyn Any { self.widget.as_any() }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self.widget.as_any_mut() }
}

impl BoxWidget {
  pub fn key(&self) -> Option<&Key> { self.downcast_attr_widget::<Key>().map(|k| k.key()) }

  pub fn downcast_attr_widget<AttrData: 'static>(&self) -> Option<&WidgetAttr<BoxWidget, AttrData>>
  where
    Self: Sized,
  {
    let mut attr = self.as_attr();

    while attr.as_ref().map_or(false, |a| {
      !a.as_any().is::<WidgetAttr<BoxWidget, AttrData>>()
    }) {
      attr = attr.and_then(|a| a.widget().as_attr());
    }

    attr.and_then(|a| a.as_any().downcast_ref())
  }

  pub fn downcast_attr_widget_mut<AttrData: 'static>(
    &mut self,
  ) -> Option<&mut WidgetAttr<BoxWidget, AttrData>>
  where
    Self: Sized,
  {
    let mut attr = self.as_attr_mut();

    while attr.as_ref().map_or(false, |a| {
      !a.as_any().is::<WidgetAttr<BoxWidget, AttrData>>()
    }) {
      attr = attr.and_then(|a| a.widget_mut().as_attr_mut());
    }

    attr.and_then(|a| a.as_any_mut().downcast_mut())
  }
}

impl Widget for BoxWidget {
  #[inline]
  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    self
  }
}

impl AttributeAttach for BoxWidget {
  type HostWidget = Self;
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

  impl AsAttr for $name {
    #[inline]
    fn as_attr(&self) -> Option<&dyn Attribute> { self.$proxy_member.as_attr() }

    #[inline]
    fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { self.$proxy_member.as_attr_mut() }
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
