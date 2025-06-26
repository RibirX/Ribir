use crate::prelude::*;

#[derive(Default)]
pub struct TrackWidgetId {
  wid: TrackId,
}

impl<'c> ComposeChild<'c> for TrackWidgetId {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let track_id = this.read().wid.clone();
    child
      .on_build(move |id| track_id.set(Some(id)))
      .attach_data(Box::new(Queryable(this.read().wid.clone())))
  }
}

impl TrackWidgetId {
  /// The WidgetId of the Widget may be changed durning running.
  /// Don't rely on the id to do things unless necessary. If you must get the
  /// id, you can use TrackId to capture the value.
  pub fn track_id(&self) -> TrackId { self.wid.clone() }
}

impl Declare for TrackWidgetId {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    reset_test_env,
    test_helper::{split_value, *},
  };

  #[test]
  fn track_id_of_pipe() {
    reset_test_env!();
    let (trigger, w_trigger) = split_value(false);
    let (id, w_id) = split_value(<Option<TrackId>>::None);
    let w = fn_widget! {
      let w = @ pipe!(*$trigger).map(move |_| fn_widget!{ @ Void {} });
      let mut w = FatObj::new(w);
      @(w) {
        on_mounted: move |_| {
          *$w_id.write() = Some($w.track_id());
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    let first_id = id.read().as_ref().map(|w| w.get());
    assert!(first_id.is_some());

    *w_trigger.write() = true;
    wnd.draw_frame();
    let second_id = id.read().as_ref().map(|w| w.get());
    assert!(second_id.is_some());
    assert_ne!(first_id, second_id);
  }
}
