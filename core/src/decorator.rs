use std::marker::PhantomData;

use crate::{
  prelude::{BuildCtx, ComposeChild, Declare, DeclareBuilder},
  state::State,
  widget::Widget,
};

/// Decorator is a widget to help a function declaration in the `widget!` macro,
/// that function should accept a widget and return another widget.
///
/// # Example
/// ```
/// # use ribir_core::prelude::*;
/// fn decorate_widget(widget: Widget) -> Widget {
///   widget! {
///     DynWidget {
///       cursor: CursorIcon::Hand,
///       dyns: widget
///     }
///   }.into_widget()
/// }
///
/// // We can apply `decorate_widget` in `Void` in a declared way.
/// let _w = widget! {
///   Decorator {
///     decorate_fn: decorate_widget,
///     Void {}
///   }
/// };
/// ```
#[derive(Declare)]
pub struct Decorator<Host, F: Fn(Host) -> Widget> {
  decorate_fn: F,
  #[declare(skip)]
  _marker: PhantomData<*const Host>,
}

impl<Host, F: Fn(Host) -> Widget> ComposeChild for Decorator<Host, F> {
  type Child = Host;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let this = match this {
      State::Stateless(this) => this,
      State::Stateful(_) => panic!("A hasn't any public fields, it should never be stateful."),
    };
    (this.decorate_fn)(child)
  }
}
