use ribir_core::prelude::*;

/// region select data
#[derive(Copy, Clone)]
pub enum PointerSelectData {
  Start(Point),
  Move { from: Point, to: Point },
  End { from: Point, to: Point },
}

/// region select event
pub type PointerSelectEvent = CustomEvent<PointerSelectData>;

/// A Widget that extends Widget to emit SelectRegionEvent
#[derive(Declare)]
pub struct PointerSelectRegion {}

fn notify_select_changed(wid: WidgetId, e: PointerSelectData, wnd: &Window) {
  wnd.bubble_custom_event(wid, e);
}

impl<'c> ComposeChild<'c> for PointerSelectRegion {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      let grab_handle = Stateful::new(None);
      let from = Stateful::new(None);
      @ $child {
        on_pointer_down: move |e| {
          *$from.write() = Some(e.position());
        },
        on_pointer_move: move |e| {
          if $grab_handle.is_none()
            && $from.is_some()
            && e.mouse_buttons() == MouseButtons::PRIMARY {
            *$grab_handle.write() = Some(GrabPointer::grab(e.current_target(), &e.window()));
            notify_select_changed(
              e.current_target(),
              PointerSelectData::Start((*$from).unwrap()),
              &e.window()
            );
          }
          if $grab_handle.is_some() {
            let from = $from.unwrap();
            notify_select_changed(
              e.current_target(),
              PointerSelectData::Move{ from, to: e.position() },
              &e.window()
            );
          } else {
            $from.write().take();
          }
        },
        on_pointer_up: move |e| {
          let from = $from.write().take();
          if $grab_handle.write().take().is_some() {
            if let Some(from) = from {
              notify_select_changed(
                e.current_target(),
                PointerSelectData::End{ from, to: e.position() },
                &e.window()
              );
            }
          }
        },
      }
    }
    .into_widget()
  }
}
