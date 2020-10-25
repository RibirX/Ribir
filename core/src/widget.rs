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
pub use stateful::{StateRef, Stateful};
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

/// The common behavior of widgets, also support to dynamic cast to special
/// widget. In most of cases, needn't implement `Widget` trait directly, and
/// implement `CombinationWidget`, `RenderWidget` instead of
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;

  #[inline]
  fn is_combination(&self) -> bool { matches!(self.classify(), WidgetClassify::Combination(_)) }

  #[inline]
  fn is_render(&self) -> bool { !matches!(self.classify(), WidgetClassify::Combination(_)) }

  /// return the some-value of `WidgetAttr` reference if the widget attached
  /// attr.
  fn as_attr(&self) -> Option<&dyn Attribute>;

  /// like `as_attr`, but return mutable reference.
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute>;

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
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Widget {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget;

  fn self_state_ref(&self, ctx: &mut BuildCtx) -> StateRef<Self>
  where
    Self: Sized,
  {
    unsafe { StateRef::new(ctx.state_attr()) }
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

pub enum WidgetClassify<'a> {
  Combination(&'a dyn CombinationWidget),
  Render(&'a dyn RenderWidgetSafety),
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
}

pub struct BoxWidget {
  pub(crate) widget: Box<dyn Widget>,
}

impl std::fmt::Debug for BoxWidget {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.widget.fmt(f) }
}

impl BoxWidget {
  fn downcast_attr_widget<AttrData: 'static>(&self) -> Option<&WidgetAttr<BoxWidget, AttrData>>
  where
    Self: Sized,
  {
    let mut attr = self.as_attr();
    while let Some(a) = attr {
      let target_attr = a.as_any().downcast_ref::<WidgetAttr<BoxWidget, AttrData>>();
      if target_attr.is_some() {
        return target_attr;
      } else {
        attr = a.widget().as_attr();
      }
    }
    None
  }
}

impl Widget for BoxWidget {
  #[inline]
  fn classify(&self) -> WidgetClassify { self.widget.classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.widget.classify_mut() }

  #[inline]
  fn as_any(&self) -> &dyn Any { self.widget.as_any() }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self.widget.as_any_mut() }

  #[inline]
  fn as_attr(&self) -> Option<&dyn Attribute> { self.widget.as_attr() }

  #[inline]
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { self.widget.as_attr_mut() }

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    self
  }
}

impl AttributeAttach for BoxWidget {
  type HostWidget = BoxWidget;
}

impl From<Box<dyn Widget>> for BoxWidget {
  #[inline]
  fn from(widget: Box<dyn Widget>) -> Self { Self { widget } }
}

/// Todo: We should auto implement Widget for CombinationWidget and
/// RenderWidget after rust specialization finished.
pub macro impl_widget_for_combination_widget(
  $ty: ty
  $(, <$($generics: tt),*>)?
  $(, where $($wty:ty : $bound: tt),*)?
) {
  impl<$($($generics ,)*)?> Widget for $ty
  where
    $($($wty: $bound), *)?
  {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }

    #[inline]
    fn as_any(&self) -> &dyn Any { self }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    #[inline]
    fn as_attr(&self) -> Option<&dyn Attribute> { None }

    #[inline]
    fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { None }
  }

  impl<$($($generics ,)*)?> AttributeAttach for $ty
  where
    $($($wty: $bound), *)?
  {
    type HostWidget = $ty;
  }
}

pub macro impl_widget_for_render_widget(
  $ty: ty
  $(, <$($generics: tt),*>)?
  $(, where $($wty:ty : $bound: tt),*)?
) {
  impl<$($($generics ,)*)?> Widget for $ty
  where
    $($($wty: $bound), *)?
  {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }

    #[inline]
    fn as_any(&self) -> &dyn Any { self }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    #[inline]
    fn as_attr(&self) -> Option<&dyn Attribute> { None }

    #[inline]
    fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { None }
  }

  impl<$($($generics ,)*)?> AttributeAttach for $ty
  where
    $($($wty: $bound), *)?
  {
    type HostWidget = $ty;
  }
}

pub macro impl_proxy_widget(
  $ty: ty,
  $base_widget: tt
  $(, <$($generics: tt),*>)?
  $(, where $($wty:ty : $bound: tt),*)?
) {
  impl<$($($generics ,)*)?> Widget for $ty
  where
    $($($wty: $bound), *)?
  {
    #[inline]
    fn classify(&self) -> WidgetClassify { self.$base_widget.classify() }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { self.$base_widget.classify_mut() }

    #[inline]
    fn as_any(&self) -> &dyn Any { self }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    #[inline]
    fn as_attr(&self) -> Option<&dyn Attribute> { self.$base_widget.as_attr() }

    #[inline]
    fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute> { self.$base_widget.as_attr_mut() }
  }

  impl<$($($generics ,)*)?> AttributeAttach for $ty
  where
    $($($wty: $bound), *)?
  {
    type HostWidget = $ty;
  }
}
