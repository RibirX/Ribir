//! Animate widgets when they are structurally mounted or disposed.
//!
//! `AnimatedPresence` wraps dynamic content and plays enter / leave presence
//! transitions when the child is structurally mounted or disposed.
//!
//! Use it whenever you need to fade in, slide in, or otherwise animate the
//! lifecycle of a widget — for example, showing/hiding a toast notification,
//! conditionally rendering a panel, or swapping list items.
//!
//! Unlike [`Animate`], which animates *property value* changes over time,
//! `AnimatedPresence` is scoped to the *structural event* (mount / dispose)
//! and does **not** affect normal property changes.
//!
//! # `AnimatedPresence` vs `AnimatedVisibility`
//!
//! Both widgets can provide enter/leave animations, but they work fundamentally
//! differently:
//!
//! * **Lifecycle**: `AnimatedPresence` animates the **actual creation and
//!   destruction** of widgets (Mount/Dispose). The widget is fully destroyed
//!   from memory after leaving. `AnimatedVisibility` keeps the widget
//!   permanently in the tree and only toggles its render/layout visibility via
//!   property.
//! * **Use Cases**:
//!   * Use **`AnimatedPresence`** for dynamically generated content (like list
//!     items, route pages, or `if condition { @Widget }` blocks) to save memory
//!     by destroying invisible widgets.
//!   * Use **`AnimatedVisibility`** for static UI components (like a persistent
//!     dropdown menu or collapsible panel) that toggle frequently, saving CPU
//!     by avoiding the cost of repeatedly building/disposing the subtree.
//!
//! # How it works
//!
//! * **Enter** — When the child is mounted, `AnimatedPresence` transitions from
//!   `cases(false)` to `cases(true)` using the optional `enter` transition.
//!
//! * **Leave** — When the child is disposed, `AnimatedPresence` detaches the
//!   widget subtree from its original location and re-attaches it to the root
//!   as an overlay, anchored at its original global position. It then
//!   transitions from `cases(true)` to `cases(false)` using the optional
//!   `leave` transition, and removes the subtree once that transition finishes.
//!
//! `cases` is required. At least one of `enter` or `leave` must be provided.
//! `interruption` controls how mid-flight reversals behave.
//!
//! # Quick start
//!
//! ```rust ignore
//! fn_widget! {
//!   let show = Stateful::new(true);
//!   let mut item = @Text { text: "Hello!" };
//!   // Grab the opacity writer *before* moving `item` into the tree.
//!   let opacity = item.opacity();
//!
//!   @AnimatedPresence {
//!     cases: cases! {
//!       state: opacity,
//!       true => 1.0,
//!       false => 0.0,
//!     },
//!     enter: EasingTransition {
//!       easing: easing::LINEAR,
//!       duration: Duration::from_millis(200),
//!     },
//!     leave: EasingTransition {
//!       easing: easing::LINEAR,
//!       duration: Duration::from_millis(200),
//!     },
//!     interruption: Interruption::Fluid,
//!     @ { pipe!(if *$read(show) { item } else { @Void {} }) }
//!   }
//! }
//! ```

use ribir_core::{prelude::*, wrap_render::*};

use super::{AnimateMatch, Interruption, MatchCases, OptionalTransitionSelector};

/// Structural animation container that plays enter/leave transitions when its
/// child is mounted or disposed.
///
/// `AnimatedPresence` is ideal for widgets with dynamic lifecycles, such as
/// list items, toast notifications, or conditional blocks. When a child is
/// disposed, `AnimatedPresence` keeps it alive as a "ghost" until its leave
/// animation finishes.
pub struct AnimatedPresence<S: AnimateState + 'static> {
  present: Stateful<bool>,
  animate_match: AnimateMatch<bool, S>,
}

pub struct AnimatedPresenceDeclarer<S: AnimateState + 'static> {
  fat_obj: FatObj<()>,
  cases: Option<MatchCases<bool, S>>,
  enter: Option<Box<dyn Transition>>,
  leave: Option<Box<dyn Transition>>,
  interruption: Option<Interruption>,
}

impl<S: AnimateState + 'static> Declare for AnimatedPresence<S> {
  type Builder = AnimatedPresenceDeclarer<S>;

  fn declarer() -> Self::Builder {
    AnimatedPresenceDeclarer {
      fat_obj: FatObj::new(()),
      cases: None,
      enter: None,
      leave: None,
      interruption: None,
    }
  }
}

/// Macro to create an [`AnimatedPresence`] as the root of a function widget.
#[macro_export]
macro_rules! animated_presence {
  ($($t: tt)*) => { fn_widget! { @AnimatedPresence { $($t)* } } };
}
pub use animated_presence;

impl<S> AnimatedPresenceDeclarer<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  pub fn with_cases(&mut self, cases: MatchCases<bool, S>) -> &mut Self {
    assert!(self.cases.is_none(), "AnimatedPresence: `cases` is already set");
    self.cases = Some(cases);
    self
  }

  pub fn with_enter(&mut self, transition: impl Transition + 'static) -> &mut Self {
    assert!(self.enter.is_none(), "AnimatedPresence: `enter` is already set");
    self.enter = Some(transition.into_box());
    self
  }

  pub fn with_leave(&mut self, transition: impl Transition + 'static) -> &mut Self {
    assert!(self.leave.is_none(), "AnimatedPresence: `leave` is already set");
    self.leave = Some(transition.into_box());
    self
  }

  pub fn with_interruption(&mut self, interruption: Interruption) -> &mut Self {
    assert!(self.interruption.is_none(), "AnimatedPresence: `interruption` is already set");
    self.interruption = Some(interruption);
    self
  }
}

impl<S> ObjDeclarer for AnimatedPresenceDeclarer<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  type Target = FatObj<Stateful<AnimatedPresence<S>>>;

  fn finish(self) -> Self::Target {
    let enter = self.enter;
    let leave = self.leave;
    assert!(
      enter.is_some() || leave.is_some(),
      "AnimatedPresence requires at least one of `enter` or `leave` transition."
    );

    let present = Stateful::new(false);
    let animate_match = AnimateMatch::observe(
      present.clone_watcher(),
      self
        .cases
        .expect("AnimatedPresence requires `cases`"),
      OptionalTransitionSelector::new(move |from: &bool, to: &bool| match (*from, *to) {
        (false, true) => enter.clone(),
        (true, false) => leave.clone(),
        _ => None,
      }),
      self.interruption.unwrap_or_default(),
    );

    self
      .fat_obj
      .map(|_| Stateful::new(AnimatedPresence { present, animate_match }))
  }
}

impl<S: AnimateState + 'static> std::ops::Deref for AnimatedPresenceDeclarer<S> {
  type Target = FatObj<()>;

  fn deref(&self) -> &Self::Target { &self.fat_obj }
}

impl<S: AnimateState + 'static> std::ops::DerefMut for AnimatedPresenceDeclarer<S> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.fat_obj }
}

impl<'c, S> ComposeChild<'c> for AnimatedPresence<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let (present, animate_match) = {
      let host = this.read();
      (host.present.clone_writer(), host.animate_match.clone())
    };

    let mut child = FatObj::new(child);

    // Trigger enter animation when the widget is structurally mounted.
    let mounted_present = present.clone_writer();
    child.on_mounted(move |_| *mounted_present.write() = true);

    // Handle the leave animation when the widget is structurally disposed.
    // We capture its last visual state and re-mount it as a "ghost" overlay.
    child.on_disposing(move |e| {
      let id = e.current_target();
      let wnd = e.window();

      // If the widget was never laid out, we skip the leave animation.
      if wnd.widget_size(id).is_none() {
        return;
      }

      let pos = e.map_to_global(Point::zero());
      let running = animate_match.running_watcher();
      let ghost =
        PinnedGhost::combine_child(Stateful::new(PinnedGhost { pos }), e.preserve().reinsert());

      // Mount the ghost to the window root.
      let handle = wnd.mount(ghost);
      *present.write() = false;

      // Ensure the ghost is cleaned up once the leave animation finishes.
      wnd.once_frame_finished(move || {
        if *running.read() {
          handle.retain_until(running);
        } else {
          handle.close();
        }
      });
    });

    child.into_widget()
  }
}

/// A wrapper that anchors a widget to a fixed global position, used to keep
/// "ghost" widgets correctly positioned during leave animations after they've
/// been detached from their original parent.
struct PinnedGhost {
  pos: Point,
}

impl WrapRender for PinnedGhost {
  fn size_affected_by_child(&self, _host: &dyn Render) -> bool { false }

  fn adjust_position(&self, host: &dyn Render, _pos: Point, ctx: &mut PlaceCtx) -> Point {
    host.adjust_position(self.pos, ctx)
  }

  fn hit_test(&self, _host: &dyn Render, _ctx: &mut HitTestCtx, _pos: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Position }
}

#[cfg(test)]
mod tests {
  use std::time::Instant;

  use ribir_core::{reset_test_env, test_helper::*, window::WindowFlags};

  use super::*;
  use crate::prelude::*;

  const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);

  fn eventually(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;

    loop {
      if predicate() {
        return true;
      }
      if Instant::now() >= deadline {
        return false;
      }
      std::thread::sleep(TEST_POLL_INTERVAL);
    }
  }

  #[test]
  fn leave_animation_keeps_widget_alive() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();
    let mounted_id = Stateful::new(None::<WidgetId>);
    let c_mounted_id = mounted_id.clone_reader();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox {
            margin: EdgeInsets { left: 24., top: 18., ..EdgeInsets::ZERO },
            size: Size::new(100., 30.),
            on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(100),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let mounted_id = c_mounted_id
      .read()
      .expect("child should be mounted before triggering leave");
    let original_pos = wnd.map_to_global(Point::zero(), mounted_id);

    let root = wnd.root();
    let children_before = wnd.children_count(root);

    *c_show.write() = false;
    wnd.draw_frame();

    let children_after = wnd.children_count(root);
    assert!(
      children_after > children_before,
      "expected ghost widget appended to root (before={children_before}, after={children_after})"
    );

    let ghost_id = wnd
      .children(wnd.root())
      .last()
      .expect("ghost widget should be appended to root");
    assert_eq!(wnd.widget_pos(ghost_id), Some(original_pos));
  }

  #[test]
  fn enter_animation_runs_on_mount() {
    reset_test_env!();

    let w = fn_widget! {
      let mut item = @MockBox { size: Size::new(100., 30.) };
      let opacity = item.opacity();

      @AnimatedPresence {
        cases: cases! {
          state: opacity,
          true => 1.0,
          false => 0.0,
        },
        enter: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(100),
        },
        @ { item }
      }
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
  }

  #[test]
  fn enter_animation_starts_from_initial_case_on_mount() {
    reset_test_env!();

    let show = Stateful::new(false);
    let c_show = show.clone_writer();
    let animated_opacity = Stateful::new(0.0_f32);
    let observed_opacity = Stateful::new(-1.0_f32);
    let observed_opacity_reader = observed_opacity.clone_reader();

    let w = fn_widget! {
      let animated_opacity = animated_opacity.clone_writer();
      let observed_opacity = observed_opacity.clone_writer();
      pipe!(*$read(show)).map(move |visible| {
        let animated_opacity = animated_opacity.clone_writer();
        let observed_opacity = observed_opacity.clone_writer();
        visible.then(move || {
          @AnimatedPresence {
            cases: cases! {
              state: animated_opacity.clone_writer(),
              true => 1.0,
              false => 0.0,
            },
            enter: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(100),
            },
            @MockBox {
              size: Size::new(100., 30.),
              opacity: pipe!(*$read(animated_opacity)),
              on_performed_layout: move |_| *$write(observed_opacity) = *$read(animated_opacity),
            }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();

    *c_show.write() = true;
    wnd.draw_frame();

    assert!(
      *observed_opacity_reader.read() <= 0.01,
      "enter animation should start from an effectively hidden value on the first mounted frame, \
       got {}",
      *observed_opacity_reader.read()
    );
  }

  #[test]
  fn leave_animation_stops_cleanly_when_window_disposes() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox { size: Size::new(100., 30.) };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(100),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();

    *c_show.write() = false;
    wnd.draw_frame();
    wnd.dispose();
  }

  #[test]
  fn enter_only_dispose_does_not_keep_ghost() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox { size: Size::new(100., 30.) };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            enter: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(80),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let root = wnd.root();
    let children_before = wnd.children_count(root);

    *c_show.write() = false;
    wnd.draw_frame();

    assert_eq!(
      wnd.children_count(root),
      children_before,
      "enter-only presence should not keep a preserved ghost after dispose"
    );
  }

  #[test]
  #[should_panic(expected = "AnimatedPresence requires at least one of `enter` or `leave`")]
  fn animated_presence_requires_at_least_one_transition() {
    reset_test_env!();

    let w = fn_widget! {
      let mut item = @MockBox { size: Size::new(100., 30.) };
      let opacity = item.opacity();

      @AnimatedPresence {
        cases: cases! {
          state: opacity,
          true => 1.0,
          false => 0.0,
        },
        @ { item }
      }
    };

    let _wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
  }

  #[test]
  fn leave_animation_removes_ghost_after_completion() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox { size: Size::new(100., 30.) };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(80),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let root = wnd.root();
    let children_before = wnd.children_count(root);

    *c_show.write() = false;
    wnd.draw_frame();
    assert!(wnd.children_count(root) > children_before, "ghost should be mounted during leave");

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        wnd.children_count(root) == children_before
      }),
      "ghost should be removed after leave finishes"
    );
  }

  #[test]
  fn rapid_hide_show_hide_does_not_leave_stale_ghost() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @Container {
            margin: EdgeInsets { left: 24., top: 18., ..EdgeInsets::ZERO },
            size: Size::new(100., 30.),
            background: Color::from_rgb(255, 100, 150),
          };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            enter: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(80),
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(80),
            },
            interruption: Interruption::Fluid,
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let root = wnd.root();
    let children_before = wnd.children_count(root);

    *c_show.write() = false;
    wnd.draw_frame();
    assert!(wnd.children_count(root) > children_before, "first hide should create a ghost");

    std::thread::sleep(Duration::from_millis(20));
    *c_show.write() = true;
    wnd.draw_frame();

    std::thread::sleep(Duration::from_millis(20));
    *c_show.write() = false;
    wnd.draw_frame();

    let mut children_now = wnd.children_count(root);
    for _ in 0..10 {
      std::thread::sleep(Duration::from_millis(30));
      wnd.draw_frame();
      children_now = wnd.children_count(root);
      if children_now == children_before {
        break;
      }
    }

    assert_eq!(
      children_now, children_before,
      "rapid hide/show/hide should not leave a stale preserved ghost",
    );
  }

  #[test]
  fn hide_show_hide_after_leave_completion_still_creates_new_leave() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox {
            margin: EdgeInsets { left: 24., top: 18., ..EdgeInsets::ZERO },
            size: Size::new(100., 30.),
          };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            enter: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(60),
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(60),
            },
            interruption: Interruption::Fluid,
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let root = wnd.root();
    let children_before = wnd.children_count(root);

    *c_show.write() = false;
    wnd.draw_frame();
    assert!(wnd.children_count(root) > children_before, "first hide should create a ghost");

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        wnd.children_count(root) == children_before
      }),
      "first leave should finish and remove the ghost"
    );

    *c_show.write() = true;
    wnd.draw_frame();
    std::thread::sleep(Duration::from_millis(20));
    wnd.draw_frame();

    *c_show.write() = false;
    wnd.draw_frame();
    assert!(
      wnd.children_count(root) > children_before,
      "second hide after remount should create a new ghost",
    );
  }

  #[test]
  fn double_dispose_loop() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();
    let mounted_id = Stateful::new(None::<WidgetId>);
    let c_mounted_id = mounted_id.clone_reader();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        visible.then(move || {
          let mut item = @MockBox {
            margin: EdgeInsets { left: 24., top: 18., ..EdgeInsets::ZERO },
            size: Size::new(100., 30.),
            on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          };
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(100),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let mounted_id = c_mounted_id
      .read()
      .expect("child should be mounted before triggering leave");

    *c_show.write() = false;
    wnd.draw_frame();

    wnd.dispose_widget(mounted_id);
    wnd.draw_frame();

    let children_count = wnd.children_count(wnd.root());
    assert!(children_count <= 2, "Should not create multiple ghosts");
  }

  struct PainterHit(Stateful<i32>);

  impl Render for PainterHit {
    fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.max }

    fn paint(&self, _ctx: &mut PaintingCtx) { *self.0.write() += 1; }
  }

  #[test]
  fn leave_animation_keeps_painting_during_ghost_lifetime() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();
    let hit = Stateful::new(0);
    let c_hit = hit.clone_reader();

    let w = fn_widget! {
      let hit = hit.clone_writer();
      pipe!(*$read(show)).map(move |visible| {
        let hit = hit.clone_writer();
        visible.then(move || {
          let mut item = FatObj::new(PainterHit(hit));
          let opacity = item.opacity();

          @AnimatedPresence {
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            enter: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(80),
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(120),
            },
            @ { item }
          }
        })
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let painted_before_hide = *c_hit.read();

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        *c_hit.read() > painted_before_hide
      }),
      "enter path should paint the widget"
    );
    let painted_after_enter = *c_hit.read();

    *c_show.write() = false;
    wnd.draw_frame();

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        *c_hit.read() > painted_after_enter
      }),
      "ghost should still paint when leave starts"
    );
    let painted_during_leave_start = *c_hit.read();

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        *c_hit.read() > painted_during_leave_start
      }),
      "ghost should keep painting during leave instead of freezing"
    );
    let painted_mid_leave = *c_hit.read();

    assert!(painted_after_enter > painted_before_hide, "enter path should paint the widget");
    assert!(
      painted_during_leave_start > painted_after_enter,
      "ghost should still paint when leave starts"
    );
    assert!(
      painted_mid_leave > painted_during_leave_start,
      "ghost should keep painting during leave instead of freezing"
    );
  }
}
