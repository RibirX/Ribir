use ribir_core::prelude::*;

use crate::prelude::*;

/// The `Switch` widget toggles the state of a single item on or off.
///
/// # Example
///
/// ```
/// use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// let _switch = switch! { checked: true };
/// ```
///
/// It also supports a label before or after the switch.
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let _switch = switch! {
///   @ { "Default label placed after the switch!" }
/// };
///
/// let _leading = switch! {
///   @Leading::new("Label placed before the switch!")
/// };
///
/// let _trailing = switch! {
///   @Trailing::new("Label placed after the switch!")
/// };
/// ```
#[derive(Clone, Copy, Declare, PartialEq, Eq)]
pub struct Switch {
  #[declare(default, event = Switch.checked)]
  pub checked: bool,
}

pub type SwitchChanged = CustomEvent<Switch>;

class_names! {
  /// The class name for the container of a checked switch.
  SWITCH_CHECKED,
  /// The class name for the container of an unchecked switch.
  SWITCH_UNCHECKED,
  /// The base class name for the switch container.
  SWITCH,
  /// The base class name for the switch thumb.
  SWITCH_THUMB,
  /// The class name for the thumb of a checked switch.
  SWITCH_THUMB_CHECKED,
  /// The class name for the thumb of an unchecked switch.
  SWITCH_THUMB_UNCHECKED,
}

impl Switch {
  fn state_class_name(&self) -> ClassName {
    if self.checked { SWITCH_CHECKED } else { SWITCH_UNCHECKED }
  }

  fn thumb_class_name(&self) -> ClassName {
    if self.checked { SWITCH_THUMB_CHECKED } else { SWITCH_THUMB_UNCHECKED }
  }

  fn request_toggle(&self, e: &CommonEvent) {
    let mut new_state = *self;
    new_state.switch_check();
    e.window()
      .bubble_custom_event(e.target(), new_state);
  }

  /// Manually toggle the switch state.
  /// This is an imperative API and should not be used for default UI
  /// interactions.
  pub fn switch_check(&mut self) { self.checked = !self.checked; }
}

impl ComposeChild<'static> for Switch {
  type Child = Option<PositionChild<TextValue>>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    let init_thumb_class = this.read().thumb_class_name();
    fn_widget! {
      let switch_widget = @Stack {
          class: distinct_pipe!([SWITCH, $read(this).state_class_name()]),
          @Container {
            hint_size: Size::new(40., 20.),
            class: class_list![
              SWITCH_THUMB,
              distinct_pipe!(Some($read(this).thumb_class_name()))
                .with_init_value(Some(init_thumb_class)),
            ],
          }
      };

      @FatObj {
        cursor: CursorIcon::Pointer,
        on_action: move |e| $read(this).request_toggle(e),
        @ { icon_with_label(switch_widget.into_widget(), child) }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use ribir_core::{prelude::*, test_helper::*};
  use ribir_dev_helper::*;
  use smallvec::smallvec;

  use super::*;

  static SWITCH_THUMB_MOUNT_COUNT: AtomicUsize = AtomicUsize::new(0);

  #[test]
  fn switch_thumb_base_class_is_not_remounted_on_toggle() {
    reset_test_env!();
    SWITCH_THUMB_MOUNT_COUNT.store(0, Ordering::SeqCst);

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let mut classes = Classes::default();
        classes.insert(SWITCH, |w| w);
        classes.insert(SWITCH_CHECKED, |w| w);
        classes.insert(SWITCH_UNCHECKED, |w| w);
        classes.insert(SWITCH_THUMB_CHECKED, |w| w);
        classes.insert(SWITCH_THUMB_UNCHECKED, |w| w);
        classes.insert(SWITCH_THUMB, |w| {
          fn_widget! {
            @FatObj {
              on_mounted: move |_| {
                SWITCH_THUMB_MOUNT_COUNT.fetch_add(1, Ordering::SeqCst);
              },
              @ { w }
            }
          }
          .into_widget()
        });

        @Providers {
          providers: smallvec![Provider::new(classes)],
          @Switch {}
        }
      },
      Size::new(120., 120.),
    );
    wnd.draw_frame();
    assert_eq!(SWITCH_THUMB_MOUNT_COUNT.load(Ordering::SeqCst), 1);

    wnd.process_cursor_move(Point::new(20., 20.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();

    assert_eq!(SWITCH_THUMB_MOUNT_COUNT.load(Ordering::SeqCst), 1);
  }
  widget_image_tests!(
    switch,
    WidgetTester::new(self::column! {
      @Switch { checked: true, @ { "checked" } }
      @Switch { @Trailing::new("unchecked") }
    })
    .with_wnd_size(Size::new(240., 160.)),
  );
}
