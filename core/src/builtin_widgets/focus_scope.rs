use crate::{events::focus_mgr::FocusType, prelude::*};

#[derive(Declare, Query, Clone, Default)]
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
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @ $child {
        on_mounted: move |e| e.window().add_focus_node(e.id, false, FocusType::Scope),
        on_disposed: move|e| e.window().remove_focus_node(e.id, FocusType::Scope),
      }
      .widget_build(ctx!())
      .attach_state_data(this, ctx!())
    }
  }
}

#[cfg(test)]
mod tests {
  use winit::{
    dpi::LogicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
  };

  use super::*;
  use crate::{test_helper::*, window::DelayEvent};

  #[test]
  fn tab_scope() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let size = Size::zero();
    let widget = fn_widget! {
      @MockMulti {
        @MockBox { size, tab_index: 0i16, auto_focus: true }
        @FocusScope {
          skip_descendants: false,
          tab_index: 3i16,
          @MockMulti {
            @MockBox { size, tab_index: 1i16, }
            @MockBox { size, tab_index: 2i16, }
            @MockBox { size, tab_index: 3i16, }
          }
        }
        @MockBox { size, tab_index: 1i16 }
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
    let widget = fn_widget! {
      @MockMulti {
        @MockBox { size, tab_index: 0i16, auto_focus: true }
        @FocusScope {
          can_focus: true,
          skip_descendants: true,
          tab_index: 3i16,
          @MockMulti {
            @MockBox { size, tab_index: 1i16, }
            @MockBox { size, tab_index: 2i16, }
            @MockBox { size, tab_index: 3i16, }
          }
        }
        @MockBox { size, tab_index: 1i16 }
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
    let tap_cnt = Stateful::new(0);
    let result = tap_cnt.clone_reader();
    let widget = fn_widget! {
      let mut host = @FocusScope {
        can_focus: false,
        on_key_down: move |_| *$tap_cnt.write() += 1,
      };
      let request_focus_box = @MockBox {
        size,
        on_pointer_down: move |_| $host.request_focus()
      };
      @MockMulti {
        @$host {
          @MockMulti {
            @MockBox { size, on_key_down: move |_| *$tap_cnt.write() += 1, }
          }
        }
        @ { request_focus_box }
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
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
    });

    // will deal key event twice (inner and host).
    wnd.draw_frame();

    wnd.add_delay_event(DelayEvent::KeyDown {
      id: wnd.focusing().unwrap(),
      physical_key: PhysicalKey::Code(KeyCode::Digit1),
      key: VirtualKey::Character("1".into()),
    });

    wnd.run_frame_tasks();
    wnd.draw_frame();
    assert_eq!(*result.read(), 2);
  }
}
