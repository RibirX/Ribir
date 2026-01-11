use ribir_core::prelude::*;

/// region select data
#[derive(Copy, Clone)]
pub enum PointerSelectData {
  Start(Point),
  Move { from: Point, to: Point },
  End { from: Point, to: Point },
}

impl PointerSelectData {
  pub fn endpoints(&self) -> (Point, Point) {
    match self {
      PointerSelectData::Start(p) => (*p, *p),
      PointerSelectData::Move { from, to } | PointerSelectData::End { from, to } => (*from, *to),
    }
  }
}

/// region select event
pub type PointerSelectEvent = CustomEvent<PointerSelectData>;

/// A Widget that extends Widget to emit SelectRegionEvent
#[declare]
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
      @(child) {
        on_pointer_down: move |e| {
          *$write(from) = Some(e.position());
        },
        on_pointer_move: move |e| {
          let mut grab_handle = $write(grab_handle);
          let mut from = $write(from);
          if grab_handle.is_none()
            && from.is_some()
            && e.mouse_buttons() == MouseButtons::PRIMARY {
            *grab_handle = Some(GrabPointer::grab(e.current_target(), &e.window()));
            notify_select_changed(
              e.current_target(),
              PointerSelectData::Start((*from).unwrap()),
              &e.window()
            );
          }
          if grab_handle.is_some() {
            let from = from.unwrap();
            notify_select_changed(
              e.current_target(),
              PointerSelectData::Move{ from, to: e.position() },
              &e.window()
            );
          } else {
            from.take();
          }
        },
        on_pointer_up: move |e| {
          let from = $write(from).take();
          if $write(grab_handle).take().is_some()
            && let Some(from) = from
          {
            notify_select_changed(
              e.current_target(),
              PointerSelectData::End{ from, to: e.position() },
              &e.window()
            );
          }
        },
      }
    }
    .into_widget()
  }
}
