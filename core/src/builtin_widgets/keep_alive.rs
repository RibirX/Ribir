use crate::prelude::*;

/// A wrapper that keeps a widget alive (paintable and layout-capable) after
/// it has been logically disposed until `keep_alive` becomes `false` or the
/// parent is dropped.
///
/// This does not change the widget lifecycle semantics — the widget is
/// removed from the tree when disposed — but when `keep_alive` is true the
/// object is retained (not immediately dropped) so it can continue to run
/// animations or paint its final frame.
///
/// This is useful to play exit/leave animations when a widget is removed.
///
/// This is a built-in `FatObj` field. Setting `keep_alive` attaches `KeepAlive`
/// behavior to the host.
///
/// # Example
///
/// Use `keep_alive` to allow a widget to run a leave animation before being
/// dropped.
///
/// ``` rust
/// use ribir::{material::md, prelude::*};
///
/// fn_widget! {
///   let checkbox = @Checkbox { checked: true };
///   let text = pipe!($read(checkbox).checked).map(move |v|
///     if v {
///       let mut w = @Text {
///         text: "text will survive even if removed until animation finished!",
///       };
///       // opacity animation will auto trigger when the opacity is changed
///       let animate = w.opacity()
///         .transition(EasingTransition{
///           easing: easing::LINEAR,
///           duration: md::easing::duration::SHORT4,
///         }.box_it());
///       @(w) {
///         // when the widget alive(the opacity is set to 0 in on_disposed)
///         // or the animation is running, keep the widget alive
///         keep_alive: pipe!(*$read(w.opacity()) != 0. || $read(animate).is_running()),
///         on_disposed: move |_| *$write(w.opacity()) = 0.,
///       }.into_widget()
///     } else {
///       @Void {}.into_widget()
///     }
///   );
///   @Row {
///     align_items: Align::Center,
///     @ { checkbox }
///     @ { text }
///   }
/// };
/// ```
#[derive(Default)]
pub struct KeepAlive {
  pub keep_alive: bool,
  wid: TrackId,
}

impl Declare for KeepAlive {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for KeepAlive {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let track = TrackWidgetId { wid: this.read().track_id() };

    track
      .with_child(child)
      .into_widget()
      .dirty_on(this.raw_modifies(), DirtyPhase::Layout)
      .try_unwrap_state_and_attach(this)
  }
}

impl KeepAlive {
  pub(crate) fn track_id(&self) -> TrackId { self.wid.clone() }
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
      pipe!(*$read(remove_widget)).map(move |v|
        (!v).then(move || fn_widget!{
          @Void {
            keep_alive: pipe!(*$read(keep_alive))
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
