use std::cell::RefCell;

use crate::{events::focus_mgr::FocusType, prelude::*};

#[derive(Declare, Clone, Default)]
pub struct FocusScope {
  /// If true, the descendants can not be focused.
  /// Default value is false, then the hold FocusScope subtree can be focused
  #[declare(default)]
  pub skip_descendants: bool,

  /// If true (default), then the host widget can not be focused, but not skip
  /// the whole subtree if skip_descendants is false.
  /// If false, then the host widget can be focused.
  #[declare(default = true)]
  pub skip_host: bool,
}

impl<'c> ComposeChild<'c> for FocusScope {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      let guard = Sc::new(RefCell::new(None));
      let guard2 = guard.clone();
      @ $child {
        on_mounted: move |e| {
          *guard.borrow_mut() = Some(
            Window::add_focus_node(e.window(), $child.track_id(), false, FocusType::Scope)
          );
        },
        on_disposed: move|_| {
          guard2.borrow_mut().take();
        },
      }
      .into_widget()
      .try_unwrap_state_and_attach(this)
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use winit::{
    dpi::LogicalPosition,
    event::{DeviceId, ElementState, WindowEvent},
  };

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn tab_scope() {
    reset_test_env!();

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
    let tree = wnd.tree();

    focus_mgr.on_widget_tree_update(tree);

    let id0 = tree.content_root().first_child(tree).unwrap();
    let scope = id0.next_sibling(tree).unwrap();
    let scope_id1 = scope.first_child(tree).unwrap();
    let scope_id2 = scope_id1.next_sibling(tree).unwrap();
    let scope_id3 = scope_id2.next_sibling(tree).unwrap();
    let id1 = scope.next_sibling(tree).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id1));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id1));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id2));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id3));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id0));

      // previous focus sequential
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id3));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id2));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(scope_id1));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id1));
    }
  }

  #[test]
  fn tab_scope_self_only() {
    reset_test_env!();

    let size = Size::zero();
    let widget = fn_widget! {
      @MockMulti {
        @MockBox { size, tab_index: 0i16, auto_focus: true }
        @FocusScope {
          skip_host: false,
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
    let tree = wnd.tree_mut();
    focus_mgr.on_widget_tree_update(tree);

    let id0 = tree.content_root().first_child(tree).unwrap();
    let scope = id0.next_sibling(tree).unwrap();
    let id1 = scope.next_sibling(tree).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget(FocusReason::AutoFocus);
      assert_eq!(focus_mgr.focusing(), Some(id1));
      focus_mgr.focus_next_widget(FocusReason::AutoFocus);
      assert_eq!(focus_mgr.focusing(), Some(scope));
      focus_mgr.focus_next_widget(FocusReason::AutoFocus);
      assert_eq!(focus_mgr.focusing(), Some(id0));

      // previous focus sequential
      focus_mgr.focus_prev_widget(FocusReason::AutoFocus);
      assert_eq!(focus_mgr.focusing(), Some(scope));
      focus_mgr.focus_prev_widget(FocusReason::AutoFocus);
      assert_eq!(focus_mgr.focusing(), Some(id1));
    }
  }

  #[test]
  fn focus_scope() {
    reset_test_env!();

    let size = Size::new(50., 50.);
    let tap_cnt = Stateful::new(0);
    let result = tap_cnt.clone_reader();
    let widget = fn_widget! {
      let mut host = @FocusScope {
        skip_host: true,
        on_key_down: move |_| *$tap_cnt.write() += 1,
      };
      let request_focus_box = @MockBox {
        size,
        on_pointer_down: move |_| $host.request_focus(FocusReason::Pointer)
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

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: DeviceId::dummy(),
      position: LogicalPosition::new(75., 25.).to_physical(1.),
    });
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);

    // will deal key event twice (inner and host).
    wnd.draw_frame();

    wnd.processes_keyboard_event(
      PhysicalKey::Code(KeyCode::Digit0),
      VirtualKey::Character("0".into()),
      false,
      KeyLocation::Standard,
      ElementState::Pressed,
    );

    wnd.run_frame_tasks();
    wnd.draw_frame();
    assert_eq!(*result.read(), 2);
  }
}
