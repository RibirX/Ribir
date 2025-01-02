use ribir_core::prelude::*;

/// region select data
#[derive(Clone, Copy)]
pub enum SelectRegionData {
  SelectRect { from: Point, to: Point },
  DoubleSelect(Point),
  ShiftTo(Point),
  SetTo(Point),
}

/// region select event
pub type SelectRegionEvent = CustomEvent<SelectRegionData>;

/// A Widget that extends Widget to emit SelectRegionEvent
#[derive(Declare)]
pub struct SelectRegion {}

fn notify_select_changed(wid: WidgetId, e: SelectRegionData, wnd: &Window) {
  wnd.bubble_custom_event(wid, e);
}

impl<'c> ComposeChild<'c> for SelectRegion {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let child = FatObj::new(child);
      let grab_handle = Stateful::new(None);
      let from = Stateful::new(None);
      @ $child {
        on_pointer_down: move |e| {
          if e.with_shift_key() {
            notify_select_changed(
              e.current_target(),
              SelectRegionData::ShiftTo(e.position()),
              &e.window()
            );
          } else {
            notify_select_changed(
              e.current_target(),
              SelectRegionData::SetTo(e.position()),
              &e.window()
            );
          }
          *$from.write() = Some(e.position());
        },
        on_pointer_move: move |e| {
          if $grab_handle.is_none()
            && $from.is_some()
            && e.mouse_buttons() == MouseButtons::PRIMARY {
            *$grab_handle.write() = Some(GrabPointer::grab(e.current_target(), &e.window()));
          }
          if $grab_handle.is_some() {
            let from = $from.unwrap();
            notify_select_changed(
              e.current_target(),
              SelectRegionData::SelectRect { from, to: e.position() },
              &e.window()
            );
          } else {
            $from.write().take();
          }
        },
        on_pointer_up: move |_| {
          $from.write().take();
          $grab_handle.write().take();
        },
        on_double_tap: move |e| {
          notify_select_changed(
            e.current_target(),
            SelectRegionData::DoubleSelect(e.position()),
            &e.window()
          );
        }
      }
    }
    .into_widget()
  }
}
