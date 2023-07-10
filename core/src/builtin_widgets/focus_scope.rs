use crate::{events::focus_mgr::FocusType, impl_query_self_only, prelude::*};

#[derive(Declare, Clone, Default)]
pub struct FocusScope {
  /// If true, the descendants can not be focused.
  /// Defalut value is false, then the hold FocusScope subtree can be focused
  #[declare(default)]
  pub skip_descendants: bool,

  /// If true, the child widget can be focused.
  /// Defalut value is false, then the child widget can't be focused
  #[declare(default)]
  pub can_focus: bool,
}

impl ComposeChild for FocusScope {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let w = widget! {
      DynWidget {
        dyns: child,
        on_mounted: move |ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .add_focus_node(ctx.id, false, FocusType::Scope, ctx.tree_arena()),
        on_disposed: move|ctx| WidgetCtxImpl::wnd_ctx(&ctx)
          .remove_focus_node(ctx.id, FocusType::Scope),
      }
    };
    compose_child_as_data_widget(w, this)
  }
}

impl Query for FocusScope {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use winit::{
    dpi::LogicalPosition,
    event::{DeviceId, ElementState, KeyboardInput, MouseButton, WindowEvent},
  };

  use super::*;
  use crate::test_helper::*;

  #[test]
  fn tab_scope() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

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

    let mut wnd = TestWindow::new(widget);
    let Window { dispatcher, widget_tree, .. } = &mut *wnd;
    dispatcher.refresh_focus(widget_tree);

    let arena = &widget_tree.arena;
    let id0 = widget_tree.root().first_child(arena).unwrap();
    let scope = id0.next_sibling(arena).unwrap();
    let scope_id1 = scope.first_child(arena).unwrap();
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

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

    let mut wnd = TestWindow::new(widget);
    let Window { dispatcher, widget_tree, .. } = &mut *wnd;
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

  #[test]
  fn focus_scope() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let size = Size::new(50., 50.);
    let tap_cnt = Rc::new(RefCell::new(0));
    let result = tap_cnt.clone();
    let widget = widget! {
      init {
        let tap_cnt2 = tap_cnt.clone();
      }
      MockMulti {
        FocusScope {
          id: host,
          can_focus: false,
          on_key_down: move |_| *tap_cnt.borrow_mut() += 1,
          MockMulti {
            MockBox { size, on_key_down: move |_| *tap_cnt2.borrow_mut() += 1, }
          }
        }
        MockBox { size, on_pointer_down: move |_| host.request_focus(),}
      }
    };

    let mut wnd = TestWindow::new(widget);
    wnd.draw_frame();

    // request_focus
    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: LogicalPosition::new(75., 25.).to_physical(1.),
      modifiers: ModifiersState::default(),
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    // will deal key event twice (inner and host).
    wnd.draw_frame();
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::KeyboardInput {
      device_id: unsafe { DeviceId::dummy() },
      input: KeyboardInput {
        scancode: 0,
        virtual_keycode: Some(VirtualKeyCode::A),
        state: ElementState::Pressed,
        modifiers: ModifiersState::default(),
      },
      is_synthetic: false,
    });

    wnd.draw_frame();
    assert_eq!(*result.borrow(), 2);
  }
}
