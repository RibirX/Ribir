use crate::{
  data_widget::compose_child_as_data_widget,
  events::focus_mgr::{FocusHandle, FocusType},
  impl_query_self_only,
  prelude::*,
};

#[derive(Default, Declare)]
pub struct FocusNode {
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
}

impl ComposeChild for FocusNode {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let this = this.into_writable();

    let w = widget! {
      states { this: this.clone() }
      DynWidget {
        on_mounted: move |ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .add_focus_node(ctx.id, this.auto_focus, FocusType::NODE, ctx.tree_arena()),
        on_disposed: move|ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .remove_focus_node(ctx.id, FocusType::NODE),
        dyns: child
      }
    }
    .into_widget();
    compose_child_as_data_widget(w, State::Stateful(this))
  }
}

impl Query for FocusNode {
  impl_query_self_only!();
}

fn has_focus(r: &dyn Render) -> bool {
  let mut focused = false;
  r.query_on_first_type(QueryOrder::OutsideFirst, |_: &FocusNode| focused = true);
  focused
}

pub(crate) fn dynamic_compose_focus_node(widget: Widget) -> Widget {
  match widget {
    Widget::Compose(c) => (|ctx: &BuildCtx| dynamic_compose_focus_node(c(ctx))).into_widget(),
    Widget::Render { ref render, children: _ } => {
      if has_focus(render) {
        widget
      } else {
        widget! {
          DynWidget {
            tab_index: 0,
            dyns: widget,
          }
        }
        .into_widget()
      }
    }
  }
}
#[derive(Declare)]
pub struct RequestFocus {
  #[declare(default)]
  handle: Option<FocusHandle>,
}

impl ComposeChild for RequestFocus {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let this = this.into_writable();
    let w = widget! {
      states { this: this.clone() }
      DynWidget {
        on_mounted: move |ctx| {
          this.silent().handle = Some(ctx.wnd_ctx().focus_handle(ctx.id));
        },
        dyns: child
      }
    }
    .into_widget();
    let widget = compose_child_as_data_widget(w, State::Stateful(this));
    dynamic_compose_focus_node(widget)
  }
}
impl RequestFocus {
  pub fn request_focus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.request_focus();
    }
  }

  pub fn unfocus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.unfocus();
    }
  }
}

impl Query for RequestFocus {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn dynamic_focus_node() {
    #[derive(Declare)]
    struct AutoFocusNode {}

    impl ComposeChild for AutoFocusNode {
      type Child = Widget;
      #[inline]
      fn compose_child(_this: State<Self>, child: Self::Child) -> Widget {
        dynamic_compose_focus_node(child)
      }
    }
    let widget = widget! {
      AutoFocusNode{
        AutoFocusNode{
          AutoFocusNode {
            MockBox {
              size: Size::default(),
            }
          }
        }
      }
    };

    let wnd = Window::default_mock(widget, None);
    let tree = &wnd.widget_tree;
    let id = tree.root();
    let node = id.get(&tree.arena).unwrap();
    let mut cnt = 0;
    node.query_all_type(
      |_: &FocusNode| {
        cnt += 1;
        true
      },
      QueryOrder::InnerFirst,
    );

    assert!(cnt == 1);
  }
}
