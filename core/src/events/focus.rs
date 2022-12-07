use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};
use std::cell::RefCell;

use super::focus_mgr::FocusType;

// todo: split focus listener, and auto add focus node when listen on key/char.
/// Focus attr attach to widget to support get ability to focus in.
#[derive(Default, Declare)]
pub struct FocusListener {
  /// Indicates that `widget` can be focused, and where it participates in
  /// sequential keyboard navigation (usually with the Tab key, hence the name.
  ///
  /// It accepts an integer as a value, with different results depending on the
  /// integer's value:
  /// - A negative value (usually -1) means that the widget is not reachable via
  ///   sequential keyboard navigation, but could be focused with API or
  ///   visually by clicking with the mouse.
  /// - Zero means that the element should be focusable in sequential keyboard
  ///   navigation, after any positive tab_index values and its order is defined
  ///   by the tree's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the tree source. The maximum value for tab_index is 32767.
  ///   If not specified, it takes the default value 0.
  #[declare(default, builtin)]
  pub tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial
  /// focus.
  #[declare(default, builtin)]
  pub auto_focus: bool,
  #[declare(default, builtin, convert=custom)]
  pub focus: Callback,
  #[declare(default, builtin, convert=custom)]
  pub blur: Callback,

  #[declare(default)]
  wid: Option<WidgetId>,
}


#[derive(Declare)]
pub struct FocusInOutListener {
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  #[declare(default, builtin, convert=custom)]
  pub focus_in: Callback,

  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  #[declare(default, builtin, convert=custom)]
  pub focus_out: Callback,
}


type Callback = RefCell<Option<Box<dyn for<'r> FnMut(&'r mut FocusEvent)>>>;

pub type FocusEvent = EventCommon;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusEventType {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  Focus,
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  Blur,
}

fn has_focus(r: &dyn Render) -> bool {
  let mut focused = false;
  r.query_on_first_type(QueryOrder::OutsideFirst, |_: &FocusListener| focused = true);
  focused
}

pub(crate) fn dynamic_compose_focus(widget: Widget) -> Widget {
  match widget {
    Widget::Compose(c) => (|ctx: &BuildCtx| dynamic_compose_focus(c(ctx))).into_widget(),
    Widget::Render { ref render,  children: _ } => {
      if has_focus(render) {
        widget
      } else {
        widget! {
          DynWidget {
            tab_index: 0,
            dyns: widget,
          }
        }
      }
    }
  }
}

impl ComposeChild for FocusListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let this = this.into_stateful();
    let w = widget! {
        track { this: this.clone() }
        DynWidget {
          mounted: move |ctx| {
            this.wid = Some(ctx.id);
            WidgetCtxImpl::app_ctx(&ctx).add_focus_node(ctx.id, FocusType::NODE, ctx.tree_arena());
          },
          disposed: move|ctx| {
            WidgetCtxImpl::app_ctx(&ctx).remove_focus_node(ctx.id, FocusType::NODE);
          },
        dyns: child
      }
    };
    compose_child_as_data_widget(w, StateWidget::Stateful(this))
  }
}

impl Query for FocusListener {
  impl_query_self_only!();
}

macro_rules! dispatch_event {
  ($callback: expr, $event: ident) => {
    let mut callback = $callback.borrow_mut();
    if let Some(callback) = callback.as_mut() {
      callback($event)
    }
  };
}

impl FocusListener {
  #[inline]
  pub fn dispatch_focus(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus, event);
  }

  pub fn dispatch_blur(&self, event: &mut FocusEvent) {
    dispatch_event!(self.blur, event);
  }

  pub fn request_focus(&self, ctx: &AppContext) {
    self
      .wid
      .as_ref()
      .map(|wid| ctx.focus_mgr.borrow_mut().focus_to(Some(*wid)));
  }
}

impl FocusListenerDeclarer {
  #[inline]
  pub fn focus(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus = Some(into_callback(f));
    self
  }

  #[inline]
  pub fn blur(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.blur = Some(into_callback(f));
    self
  }
}

fn into_callback(f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Callback {
  RefCell::new(Some(Box::new(f)))
}

impl FocusListener {
  #[inline]
  pub fn set_declare_focus(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.focus = into_callback(f);
  }

  #[inline]
  pub fn set_declare_blur(&mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) {
    self.blur = into_callback(f);
  }
}

impl Query for FocusInOutListener {
  impl_query_self_only!();
}

impl ComposeChild for FocusInOutListener {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl FocusInOutListener {
  #[inline]
  pub fn dispatch_focus_in(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus_in, event);
  }

  pub fn dispatch_focus_out(&self, event: &mut FocusEvent) {
    dispatch_event!(self.focus_out, event);
  }
}

impl FocusInOutListenerDeclarer {
  #[inline]
  pub fn focus_in(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus_in = Some(into_callback(f));
    self
  }

  #[inline]
  pub fn focus_out(mut self, f: impl for<'r> FnMut(&'r mut FocusEvent) + 'static) -> Self {
    self.focus_out = Some(into_callback(f));
    self
  }
}

#[derive(Declare, Clone)]
pub struct FocusScope {
  pub ignore: bool,
}

impl ComposeChild for FocusScope {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let this = this.into_stateful();
    let w = widget! {
        DynWidget {
          mounted: move |ctx| {
            WidgetCtxImpl::app_ctx(&ctx).add_focus_node(ctx.id, FocusType::SCOPE, ctx.tree_arena());
          },
          disposed: move|ctx| {
            WidgetCtxImpl::app_ctx(&ctx).remove_focus_node(ctx.id, FocusType::SCOPE);
          },
        dyns: child
      }
    };
    compose_child_as_data_widget(w, StateWidget::Stateful(this))
  }
}

impl Query for FocusScope {
  impl_query_self_only!();
}
