use ribir_core::{prelude::*, wrap_render::*};

use super::animated_presence::PresenceAction;

/// A visibility wrapper that animates a static widget's enter/leave
/// transitions.
///
/// `AnimatedVisibility` is the animated counterpart to the builtin `visible`
/// property. Use [`show`](AnimatedVisibility::show) as the single visibility
/// input when you need enter/leave animations. Unlike the builtin `visible`,
/// this wrapper can keep painting hidden content while a leave animation is
/// still running.
///
/// # Quick start
///
/// ```rust ignore
/// fn_widget! {
///   let show = Stateful::new(true);
///   let mut item = @Text { text: "Hello!" };
///   let opacity = item.opacity();
///
///   @AnimatedVisibility {
///     show: pipe!(*$show),
///     // Fade in over 200 ms when shown
///     enter: EnterAction {
///       state: opacity,
///       transition: EasingTransition {
///         easing: easing::LINEAR,
///         duration: Duration::from_millis(200),
///       },
///       from: 0.0,
///     },
///     // Fade out over 200 ms when hidden
///     leave: LeaveAction {
///       state: opacity,
///       transition: EasingTransition {
///         easing: easing::LINEAR,
///         duration: Duration::from_millis(200),
///       },
///       to: 0.0,
///     },
///     @ { item }
///   }
/// }
/// ```
///
/// # `AnimatedVisibility` vs `AnimatedPresence`
///
/// Both widgets can provide enter/leave animations, but they work fundamentally
/// differently:
///
/// * **Lifecycle**: `AnimatedVisibility` keeps the widget **permanently
///   mounted** in the tree. When `show` is false, it hides the widget from
///   layout, hit testing, and painting, but the widget still exists in memory.
///   `AnimatedPresence` hooks into structural changes, animating widgets when
///   they are actually mounted or disposed.
/// * **Use Cases**:
///   * Use **`AnimatedVisibility`** for static UI components (like persistent
///     sidebars, modals, or dropdowns) that toggle frequently. It avoids the
///     CPU overhead of allocating and building the subtree every time it
///     appears.
///   * Use **`AnimatedPresence`** for dynamically instantiated content (like
///     list items, routed pages, or true conditional `if show { @Widget }`
///     blocks) to free up memory when the widget is no longer in the DOM.
#[derive(Default, Declare)]
pub struct AnimatedVisibility {
  pub show: bool,
  #[declare(default)]
  pub enter: Option<Box<dyn PresenceAction>>,
  #[declare(default)]
  pub leave: Option<Box<dyn PresenceAction>>,
}

impl<'c> ComposeChild<'c> for AnimatedVisibility {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let wnd_id = BuildCtx::get().window().id();
    fn_widget! {
      let subscription = watch!($read(this).show)
        .distinct_until_changed()
        .subscribe(move |show| {
          if show && let Some(enter) = $read(this).enter.as_ref() {
            enter.fire(wnd_id);
          }
          if !show && let Some(leave) = $read(this).leave.as_ref() {
            leave.fire(wnd_id);
          }
        });

      @FocusScope {
        skip_descendants: pipe!(!$read(this).show),
        skip_host: pipe!(!$read(this).show),
        on_disposed: move |_| subscription.unsubscribe(),
        @AnimatedVisibility::combine_child($writer(this), child)
      }
    }
    .into_widget()
  }
}

impl WrapRender for AnimatedVisibility {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    if self.show { host.measure(clamp, ctx) } else { clamp.min }
  }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    if self.show { host.size_affected_by_child() } else { false }
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.show {
      host.paint(ctx);
    } else if self.is_leaving() {
      let mut provider = Provider::new(AllowHiddenPaint(true));
      provider.setup(ctx.as_mut());
      host.paint(ctx);
      provider.restore(ctx.as_mut());
    } else {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if self.show { host.hit_test(ctx, pos) } else { HitTest { hit: false, can_hit_child: false } }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("animated_visibility") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
      "show": self.show,
      "leaving": self.is_leaving(),
    }))
  }
}

impl AnimatedVisibility {
  fn is_leaving(&self) -> bool {
    self
      .leave
      .as_ref()
      .is_some_and(|leave| leave.is_running())
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{
    reset_test_env,
    test_helper::*,
    window::{WindowFlags, WindowId},
  };

  use super::*;
  use crate::animation::animated_presence::LeaveAction;

  struct PainterHit(Stateful<i32>);

  impl Render for PainterHit {
    fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.max }

    fn paint(&self, _ctx: &mut PaintingCtx) { *self.0.write() += 1; }
  }

  struct FireAction(Stateful<i32>);

  impl PresenceAction for FireAction {
    fn fire(&self, _window_id: WindowId) { *self.0.write() += 1; }

    fn is_running(&self) -> bool { false }

    fn running(&self) -> Box<dyn StateWatcher<Value = bool>> { Box::new(Stateful::new(false)) }
  }

  #[test]
  fn hidden_paint_runs_during_leave() {
    reset_test_env!();

    let show = Stateful::new(true);
    let hit = Stateful::new(0);
    let hit2 = hit.clone_writer();

    let w = fn_widget! {
      let mut painter = FatObj::new(PainterHit(hit2.clone_writer()));
      let opacity = painter.opacity();
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        leave: Some(LeaveAction {
          state: opacity,
          transition: EasingTransition {
            easing: easing::LINEAR,
            duration: Duration::from_millis(40),
          },
          to: 0.0,
        }.into()),
        @ { painter }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);

    *show.write() = false;
    wnd.draw_frame();
    assert!(*hit.read() >= 2, "hidden child should still paint during leave");

    std::thread::sleep(Duration::from_millis(60));
    wnd.draw_frame();
    let painted_after_leave = *hit.read();
    wnd.draw_frame();
    assert_eq!(painted_after_leave, *hit.read(), "leave end should stop hidden painting");
  }

  #[test]
  fn enter_runs_on_show_again() {
    reset_test_env!();

    let show = Stateful::new(false);
    let fire_count = Stateful::new(0);
    let fire_count_reader = fire_count.clone_reader();

    let w = fn_widget! {
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        enter: Some(Box::new(FireAction(fire_count.clone_writer())) as Box<dyn PresenceAction>),
        @MockBox { size: Size::new(100., 100.) }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    *show.write() = true;
    wnd.draw_frame();
    assert_eq!(*fire_count_reader.read(), 1);
  }
}
