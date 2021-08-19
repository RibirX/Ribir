use crate::render::*;
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
pub trait CombinationWidget: 'static {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget;

  fn box_it(self) -> BoxedWidget
  where
    Self: Sized,
  {
    BoxedWidget::Combination(Box::new(self))
  }

  /// Return the reference to the attrs that attached to the this widget.
  fn get_attrs(&self) -> Option<&Attributes> { None }
}

/// RenderWidget provide configuration for render object which provide actual
/// rendering or computing layout for the application.
pub trait RenderWidget: CloneStates + 'static {
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

  /// Return the reference to the attrs that attached to the this widget.
  fn get_attrs(&self) -> Option<&Attributes> { None }
}

/// RenderWidgetSafety is a object safety trait of RenderWidget, never directly
/// implement this trait, just implement [`RenderWidget`](RenderWidget).
pub trait RenderWidgetSafety {
  fn create_render_object(&self) -> Box<dyn RenderObjectSafety + Send + Sync>;
  fn get_attrs(&self) -> Option<&Attributes>;
  fn clone_boxed_states(&self) -> Box<dyn Any>;
}

pub enum BoxedWidget {
  Combination(Box<dyn CombinationWidget>),
  Render(Box<dyn RenderWidgetSafety>),
  SingleChild(BoxedSingleChild),
  MultiChild(BoxedMultiChild),
}

impl BoxedWidget {
  pub fn find_attr<A: Any>(&self) -> Option<&A> {
    match self {
      BoxedWidget::Combination(c) => c.find_attr(),
      BoxedWidget::Render(r) => r.find_attr(),
      BoxedWidget::SingleChild(s) => s.find_attr(),
      BoxedWidget::MultiChild(m) => m.find_attr(),
    }
  }
}

impl dyn CombinationWidget {
  pub fn find_attr<A: Any>(&self) -> Option<&A> { self.get_attrs().and_then(|attrs| attrs.get()) }
}

impl dyn RenderWidgetSafety {
  pub fn find_attr<A: Any>(&self) -> Option<&A> { self.get_attrs().and_then(|attrs| attrs.get()) }
}
