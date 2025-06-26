use crate::prelude::*;

/// A widget will be painted event if it has been dispose until the `keep_alive`
/// field be set to `false` or its parent is dropped.
///
/// This widget not effect the widget lifecycle, if the widget is dispose but
/// the `keep_alive` is `true`, it's not part of the widget tree anymore
/// but not drop immediately, is disposed in `logic`, but not release resource.
/// It's be isolated from the widget tree and can layout and paint normally.
///
/// Once the `keep_alive` field be set to `false`, the widget will be
/// dropped.
///
/// It's useful when you need run a leave animation for a widget.
#[derive(Default)]
pub struct KeepAlive {
  pub keep_alive: bool,
  pub(crate) wid: Option<TrackId>,
}

impl Declare for KeepAlive {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for KeepAlive {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut w = FatObj::new(child);
      { this.silent().wid = Some($w.track_id()); }
      w
      .into_widget()
      .dirty_on(this.raw_modifies(), DirtyPhase::Layout)
      .try_unwrap_state_and_attach(this)

    }
    .into_widget()
  }
}

impl KeepAlive {
  pub(crate) fn track_id(&self) -> Option<TrackId> { self.wid.clone() }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    let keep_alive = Stateful::new(true);
    let c_keep_alive = keep_alive.clone_writer();
    let remove_widget = Stateful::new(false);
    let c_remove_widget = remove_widget.clone_writer();
    let wnd = TestWindow::from_widget(fn_widget! {
      pipe!(*$remove_widget).map(move |v|
        (!v).then(move || fn_widget!{
          @Void {
            keep_alive: pipe!(*$keep_alive)
          }
        })
      )
    });

    let root = wnd.tree().content_root();
    wnd.draw_frame();

    *c_remove_widget.write() = true;
    wnd.draw_frame();
    assert!(!root.is_dropped(wnd.tree()));

    *c_keep_alive.write() = false;
    wnd.draw_frame();
    assert!(root.is_dropped(wnd.tree()));
  }
}
