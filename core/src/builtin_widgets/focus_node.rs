use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*, events::focus_mgr::FocusType};


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

  #[declare(default)]
  wid: Option<WidgetId>,
}

impl ComposeChild for FocusNode {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let this = this.into_stateful();
      
    let w = widget! {
      states { this: this.clone() }
      DynWidget {
        mounted: move |ctx| {
          WidgetCtxImpl::app_ctx(&ctx).add_focus_node(ctx.id, this.auto_focus, FocusType::NODE, ctx.tree_arena());
            this.clone_stateful().raw_ref().wid = Some(ctx.id);
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
impl  FocusNode {
  pub fn request_focus(&self, ctx: &AppContext) {
    self
      .wid
      .as_ref()
      .map(|wid| ctx.focus_mgr.borrow_mut().focus_to(Some(*wid)));
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
  
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn dynamic_focus_node() {

    #[derive(Declare)]
    struct AutoFockNode {
    }

    impl ComposeChild for AutoFockNode {
      type Child = Widget;
      #[inline]
      fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
        dynamic_compose_focus(child)
      }
    }
    let widget = widget! {
      AutoFockNode{
        AutoFockNode{
          AutoFockNode {
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
    node.query_all_type(|_: &FocusNode| {
      cnt += 1;
      true
    }, QueryOrder::InnerFirst);

    assert!(cnt == 1);
  }
}