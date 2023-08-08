use crate::{events::focus_mgr::FocusType, impl_query_self_only, prelude::*};

#[derive(Declare, Declare2, Clone, Default)]
pub struct FocusScope {
  /// If true, the descendants can not be focused.
  /// Default value is false, then the hold FocusScope subtree can be focused
  #[declare(default)]
  pub skip_descendants: bool,

  /// If true, the child widget can be focused.
  /// Default value is false, then the child widget can't be focused, but not
  /// skip the whole subtree.
  #[declare(default)]
  pub can_focus: bool,
}

impl ComposeChild for FocusScope {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let w = widget! {
      DynWidget {
        dyns: child,
        on_mounted: move |ctx| ctx.window().add_focus_node(ctx.id, false, FocusType::Scope),
        on_disposed: move|ctx| ctx.window().remove_focus_node(ctx.id, FocusType::Scope),
      }
    };
    DataWidget::attach_state(w.into(), this)
  }
}

impl_query_self_only!(FocusScope);

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

    let wnd = TestWindow::new(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    focus_mgr.refresh_focus();

    let tree = wnd.widget_tree.borrow();
    let arena = &tree.arena;
    let id0 = tree.root().first_child(arena).unwrap();
    let scope = id0.next_sibling(arena).unwrap();
    let scope_id1 = scope.first_child(arena).unwrap();
    let scope_id2 = scope_id1.next_sibling(arena).unwrap();
    let scope_id3 = scope_id2.next_sibling(arena).unwrap();
    let id1 = scope.next_sibling(arena).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id1));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id2));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id3));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0));

      // previous focus sequential
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id3));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id2));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope_id1));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));
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

    let wnd = TestWindow::new(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let widget_tree = wnd.widget_tree.borrow();
    focus_mgr.refresh_focus();

    let arena = &widget_tree.arena;
    let id0 = widget_tree.root().first_child(arena).unwrap();
    let scope = id0.next_sibling(arena).unwrap();
    let id1 = scope.next_sibling(arena).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0));

      // previous focus sequential
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(scope));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));
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

    wnd.run_frame_tasks();
    wnd.draw_frame();
    assert_eq!(*result.borrow(), 2);
  }
}
