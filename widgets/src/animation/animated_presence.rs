//! Animate widgets when they are structurally mounted or disposed.
//!
//! `AnimatedPresence` wraps dynamic content and plays **enter** and **leave**
//! animations automatically when the child is added to or removed from the
//! widget tree.
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
//! * **Enter** — When the child is mounted, the `enter` animation fires
//!   immediately, animating the target state from the given `from` value to the
//!   widget's real value.
//!
//! * **Leave** — When the child is disposed, `AnimatedPresence` detaches the
//!   widget subtree from its original location and re-attaches it to the root
//!   as an overlay, anchored at its original global position. The `leave`
//!   animation then plays, and the subtree is removed once it finishes.
//!
//! Both `enter` and `leave` are optional — you can specify one or both.
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
//!     // Fade in over 200 ms when mounted.
//!     enter: EnterAction {
//!       state: opacity,
//!       transition: EasingTransition {
//!         easing: easing::LINEAR,
//!         duration: Duration::from_millis(200),
//!       },
//!       from: 0.0,
//!     },
//!     // Fade out over 200 ms when disposed.
//!     leave: LeaveAction {
//!       state: opacity,
//!       transition: EasingTransition {
//!         easing: easing::LINEAR,
//!         duration: Duration::from_millis(200),
//!       },
//!       to: 0.0,
//!     },
//!     @ { pipe!(if *$read(show) { item } else { @Void {} }) }
//!   }
//! }
//! ```

use ribir_core::{animation::Animation, prelude::*, window::WindowId};

/// Type-erased presence animation action.
///
/// Both [`EnterAction`] and [`LeaveAction`] implement this trait, allowing
/// `AnimatedPresence` to store them without generics.
pub trait PresenceAction {
  /// Start the animation.
  fn fire(&self, window_id: WindowId);
  /// Whether the animation is still running.
  fn is_running(&self) -> bool;
  /// A watchable running state for reacting to animation completion.
  fn running(&self) -> Box<dyn StateWatcher<Value = bool>>;
}

/// Describes a leave animation that fires on widget disposal.
pub struct LeaveAction<S: AnimateState + 'static, T: Transition + 'static = Box<dyn Transition>> {
  /// The state to animate (e.g., an opacity writer).
  pub state: S,
  /// How to transition (easing, duration). Only used during leave.
  pub transition: T,
  /// The target value when leaving.
  pub to: S::Value,
}

/// Describes an enter animation that fires on widget mount.
pub struct EnterAction<S: AnimateState + 'static, T: Transition + 'static = Box<dyn Transition>> {
  /// The state to animate.
  pub state: S,
  /// How to transition.
  pub transition: T,
  /// The initial value to start from when entering.
  pub from: S::Value,
}

/// Structural animation container.
///
/// Wraps a dynamic child and plays enter/leave animations when the child is
/// mounted or disposed.
#[derive(Default)]
#[declare(simple)]
pub struct AnimatedPresence {
  /// Enter (mount) animation.
  #[declare(default)]
  enter: Option<Box<dyn PresenceAction>>,
  /// Leave (dispose) animation.
  #[declare(default)]
  leave: Option<Box<dyn PresenceAction>>,
}

impl<'c> ComposeChild<'c> for AnimatedPresence {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let (enter, leave) = {
      let mut w = this.write();
      (w.enter.take(), w.leave.take())
    };

    let mut obj = FatObj::new(child);
    let wnd_id = BuildCtx::get().window().id();

    // --- Enter animation ---
    if let Some(enter) = enter {
      obj.on_mounted(move |_| enter.fire(wnd_id));
    }

    // --- Leave animation ---
    //
    // On disposal the subtree is preserved, wrapped in a `GhostHost`
    // container pinned at its original global position, then mounted into the
    // window overlay layer. Because `GhostHost` is a separate Render node, the
    // child's own anchor / transform properties remain free for the leave
    // animation (e.g. a slide-out via Transform).
    if let Some(leave) = leave {
      obj.on_disposed(move |e| {
        let id = e.current_target();
        let wnd = e.window();

        // If the widget was never laid out (e.g. mounted and disposed within the
        // same frame before any layout ran), there is nothing visible to animate.
        if wnd.widget_size(id).is_none() {
          return;
        }

        // Capture the widget's original global position before mutating the
        // tree.  Must be done before any `tree_mut()` borrow.
        let pos = e.map_to_global(Point::zero());
        let preserve = e.preserve();
        let ghost = fn_widget! {
          @GhostHost {
            pos,
            @ { preserve.into_widget() }
          }
        }
        .into_widget();

        // Start the animation.
        leave.fire(wnd_id);
        wnd.mount(ghost).retain_until(leave.running());
      });
    }

    obj.into_widget()
  }
}

// ---------------------------------------------------------------------------
// Internal: ghost host
// ---------------------------------------------------------------------------

/// A transparent container allocated at disposal time to pin the leave-ghost
/// at its original global position.
///
/// `GhostHost` is a separate `Render` node so that the child's own
/// Anchor / Transform properties remain free for the leave animation.  Its
/// `adjust_position` always returns the stored position, keeping the ghost
/// fixed during `perform_place(root)`.  `size_affected_by_child` returns
/// `false` so that layout changes inside the ghost never propagate upward.
#[derive(Declare, MultiChild)]
struct GhostHost {
  pos: Point,
}

impl Render for GhostHost {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    ctx
      .perform_single_child_layout(clamp)
      .unwrap_or_default()
  }

  fn size_affected_by_child(&self) -> bool { false }

  fn place_children(&self, _: Size, _: &mut PlaceCtx) {
    // No-op: the child is already pinned at the correct global position, and
    // we don't want to adjust it during place.
  }

  fn adjust_position(&self, _pos: Point, _ctx: &mut PlaceCtx) -> Point { self.pos }

  fn hit_test(&self, _ctx: &mut HitTestCtx, _pos: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }
}

// ---------------------------------------------------------------------------
// Internal: one-shot animation wrappers
// ---------------------------------------------------------------------------

struct LeaveInner<S: AnimateState + 'static> {
  to: S::Value,
  animate: Stateful<Animate<S>>,
}

impl<S> PresenceAction for LeaveInner<S>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
{
  fn fire(&self, window_id: WindowId) {
    let mut animate = self.animate.write();
    animate.window_id = Some(window_id);
    animate.from = animate.state.get();
    animate.state.set(self.to.clone());
    animate.forget_modifies();
    drop(animate);

    self.animate.run();
  }

  fn is_running(&self) -> bool { self.animate.read().is_running() }

  fn running(&self) -> Box<dyn StateWatcher<Value = bool>> {
    Box::new(
      self
        .animate
        .part_watcher(|a| PartRef::from_value(a.is_running())),
    )
  }
}

struct EnterInner<S: AnimateState + 'static> {
  animate: Stateful<Animate<S>>,
}

impl<S> PresenceAction for EnterInner<S>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
{
  fn fire(&self, window_id: WindowId) {
    let mut animate = self.animate.write();
    animate.window_id = Some(window_id);
    animate.forget_modifies();
    drop(animate);

    self.animate.run();
  }

  fn is_running(&self) -> bool { self.animate.read().is_running() }

  fn running(&self) -> Box<dyn StateWatcher<Value = bool>> {
    Box::new(
      self
        .animate
        .part_watcher(|a| PartRef::from_value(a.is_running())),
    )
  }
}

// ---------------------------------------------------------------------------
// Conversions: user-facing → Box<dyn PresenceAction>
// ---------------------------------------------------------------------------

impl<S, T> From<LeaveAction<S, T>> for Box<dyn PresenceAction>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
  T: Transition + 'static,
{
  fn from(action: LeaveAction<S, T>) -> Self {
    let animate = {
      let mut d = Animate::declarer();
      d.with_state(action.state)
        .with_transition(action.transition.into_box())
        .with_from(action.to.clone());
      d.finish()
    };
    Box::new(LeaveInner { to: action.to, animate })
  }
}

impl<S, T> From<LeaveAction<S, T>> for Option<Box<dyn PresenceAction>>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
  T: Transition + 'static,
{
  fn from(action: LeaveAction<S, T>) -> Self { Some(action.into()) }
}

impl<S, T> From<EnterAction<S, T>> for Box<dyn PresenceAction>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
  T: Transition + 'static,
{
  fn from(action: EnterAction<S, T>) -> Self {
    let animate = {
      let mut d = Animate::declarer();
      d.with_state(action.state)
        .with_transition(action.transition.into_box())
        .with_from(action.from);
      d.finish()
    };
    Box::new(EnterInner { animate })
  }
}

impl<S, T> From<EnterAction<S, T>> for Option<Box<dyn PresenceAction>>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
  T: Transition + 'static,
{
  fn from(action: EnterAction<S, T>) -> Self { Some(action.into()) }
}

#[cfg(test)]
mod tests {
  use ribir_core::{reset_test_env, test_helper::*, window::WindowFlags};

  use super::*;

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
            leave: LeaveAction {
              state: opacity,
              transition: EasingTransition {
                easing: easing::LINEAR,
                duration: Duration::from_millis(100),
              },
              to: 0.0,
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

    // Remember how many children root had before hiding.
    let root = wnd.root();
    let children_before = wnd.children_count(root);

    // Hide the widget — leave animation should fire and ghost should be
    // re-parented to root.
    *c_show.write() = false;
    wnd.draw_frame();

    // After disposal the ghost subtree is appended to root, so root should
    // have at least one more child than before.
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
        enter: EnterAction {
          state: opacity,
          transition: EasingTransition {
            easing: easing::LINEAR,
            duration: Duration::from_millis(100),
          },
          from: 0.0,
        },
        @ { item }
      }
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
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
            leave: LeaveAction {
              state: opacity,
              transition: EasingTransition {
                easing: easing::LINEAR,
                duration: Duration::from_millis(100),
              },
              to: 0.0,
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
            leave: LeaveAction {
              state: opacity,
              transition: EasingTransition {
                easing: easing::LINEAR,
                duration: Duration::from_millis(100),
              },
              to: 0.0,
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
    wnd.draw_frame(); // Leave animation starts, id is in GhostHost

    // Manually dispose the id while it's in GhostHost!
    wnd.dispose_widget(mounted_id);
    wnd.draw_frame(); // Process disposal

    let children_count = wnd.children_count(wnd.root());
    assert!(children_count <= 2, "Should not create multiple ghosts");
  }
}
