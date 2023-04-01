use crate::{
  context::LayoutCtx, events::focus_mgr::FocusType, impl_query_self_only, prelude::*,
  widget::BoxClamp,
};

#[derive(Declare, Clone, Default)]
pub struct FocusScope {
  /// If true, the descendants can not be focused.
  /// Defalut value is false, then the FocusScope widget can be focused
  #[declare(default)]
  pub skip_descendants: bool,

  /// If true, the FocusScope can be focused.
  /// Defalut value is false, then the FocusScope widget can't be focused
  #[declare(default)]
  pub can_focus: bool,
}

impl ComposeChild for FocusScope {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let w = widget! {
      FocusScopeRender {
        on_mounted: move |ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .add_focus_node(ctx.id, false, FocusType::SCOPE, ctx.tree_arena()),
        on_disposed: move|ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .remove_focus_node(ctx.id, FocusType::SCOPE),
        DynWidget { dyns: child }
      }
    };
    compose_child_as_data_widget(w, this)
  }
}

impl Query for FocusScope {
  impl_query_self_only!();
}

#[derive(Declare, SingleChild)]
struct FocusScopeRender {}

impl Render for FocusScopeRender {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }
}

impl Query for FocusScopeRender {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn tab_scope() {
    let size = Size::zero();
    let widget = widget! {
      MockMulti {
        MockBox { size, tab_index: 0, auto_focus: true }
        FocusScope {
          skip_descendants: false,
          tab_index: 3,
          MockMulti {
            MockBox { size, tab_index: 1, }
            MockBox { size, tab_index: 2, }
            MockBox { size, tab_index: 3, }
          }
        }
        MockBox { size, tab_index: 1 }
      }
    };

    let mut wnd = Window::default_mock(widget.into_widget(), None);
    let Window { dispatcher, widget_tree, .. } = &mut wnd;
    dispatcher.refresh_focus(widget_tree);

    let arena = &widget_tree.arena;
    let id0 = widget_tree.root().first_child(arena).unwrap();
    let scope = id0.next_sibling(arena).unwrap();
    let scope_id1 = scope
      .first_child(arena)
      .unwrap()
      .first_child(arena)
      .unwrap();
    let scope_id2 = scope_id1.next_sibling(arena).unwrap();
    let scope_id3 = scope_id2.next_sibling(arena).unwrap();
    let id1 = scope.next_sibling(arena).unwrap();

    {
      // next focus sequential
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id1));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id2));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id3));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0));

      // previous focus sequential
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id3));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id2));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope_id1));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));
    }
  }

  #[test]
  fn tab_scope_self_only() {
    let size = Size::zero();
    let widget = widget! {
      MockMulti {
        MockBox { size, tab_index: 0, auto_focus: true }
        FocusScope {
          can_focus: true,
          skip_descendants: true,
          tab_index: 3,
          MockMulti {
            MockBox { size, tab_index: 1, }
            MockBox { size, tab_index: 2, }
            MockBox { size, tab_index: 3, }
          }
        }
        MockBox { size, tab_index: 1 }
      }
    };

    let mut wnd = Window::default_mock(widget.into_widget(), None);
    let Window { dispatcher, widget_tree, .. } = &mut wnd;
    dispatcher.refresh_focus(widget_tree);

    let arena = &widget_tree.arena;
    let id0 = widget_tree.root().first_child(arena).unwrap();
    let scope = id0.next_sibling(arena).unwrap();
    let id1 = scope.next_sibling(arena).unwrap();

    {
      // next focus sequential
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0));

      // previous focus sequential
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(scope));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));
    }
  }
}
