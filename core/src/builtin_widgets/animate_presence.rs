//! Animate widgets when they appear or disappear.
//!
//! `AnimatePresence` wraps dynamic content and plays **enter** and **leave**
//! animations automatically when the child is mounted or disposed.
//!
//! Use it whenever you need to fade in, slide in, or otherwise animate the
//! structural presence of a widget — for example, showing/hiding a toast,
//! toggling a panel, or swapping list items.
//!
//! Unlike [`Animate`], which animates *property value* changes over time,
//! `AnimatePresence` is scoped to the *structural event* (mount / dispose)
//! and does **not** affect normal property changes.
//!
//! # How it works
//!
//! * **Enter** — When the child is mounted, the `enter` animation fires
//!   immediately, animating the target state from the given `from` value to the
//!   widget's real value.
//!
//! * **Leave** — When the child is disposed, `AnimatePresence` detaches the
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
//!   @AnimatePresence {
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

use std::cell::{Cell, RefCell};

use ribir_algo::Rc;

use crate::{animation::Animation, prelude::*, render_helper::PureRender, window::WindowId};

/// Type-erased presence animation action.
///
/// Both [`EnterAction`] and [`LeaveAction`] implement this trait, allowing
/// `AnimatePresence` to store them without generics.
pub trait PresenceAction {
  /// Start the one-shot animation. Can only be called once.
  fn fire(&self, window_id: WindowId);
  /// Whether the animation is still running.
  fn is_running(&self) -> bool;
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
pub struct AnimatePresence {
  /// Enter (mount) animation.
  #[declare(default)]
  pub enter: Option<Box<dyn PresenceAction>>,
  /// Leave (dispose) animation.
  #[declare(default)]
  pub leave: Option<Box<dyn PresenceAction>>,
}

impl<'c> ComposeChild<'c> for AnimatePresence {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let (enter, leave) = {
      let mut w = this.write();
      (w.enter.take(), w.leave.take())
    };

    let enter = enter.map(Rc::new);
    let leave = leave.map(Rc::new);

    let mut obj = FatObj::new(child);
    let wnd_id = BuildCtx::get().window().id();

    // --- Enter animation ---
    if let Some(enter) = enter {
      obj.on_mounted(move |_| {
        enter.fire(wnd_id);
      });
    }

    // --- Leave animation ---
    //
    // On disposal the widget is wrapped in a `GhostHost` container pinned at
    // the widget's original global position.  The container is allocated as a
    // new tree node and re-attached to the tree root so the ghost keeps
    // rendering in place while the leave animation plays.  Because `GhostHost`
    // is a separate Render node, the child's own anchor / transform properties
    // remain free for the leave animation (e.g. a slide-out via Transform).
    if let Some(leave) = leave {
      let leave_fired = Rc::new(Cell::new(false));
      obj.on_disposed(move |e| {
        let id = e.current_target();
        if id.tree_parent(e.tree()).is_some() || leave_fired.replace(true) {
          return;
        }

        let wnd = e.window();

        // If the widget was never laid out (e.g. mounted and disposed within the
        // same frame before any layout ran), there is nothing visible to animate.
        if wnd.tree().store.layout_info(id).is_none() {
          return;
        }

        // Capture the widget's original global position before mutating the
        // tree.  Must be done before any `tree_mut()` borrow.
        let pos = e.map_to_global(Point::zero());

        let ghost_id;
        {
          let tree = wnd.tree_mut();
          ghost_id = tree.alloc_node(Box::new(PureRender(GhostHost { pos })));
          ghost_id.append(id, tree);
          let root = tree.root();
          root.append(ghost_id, tree);
          tree.dirty_marker().mark(root, DirtyPhase::Layout);
        }

        // Start the animation.
        leave.fire(wnd_id);

        dispose_ghost_when_leave_done(wnd_id, ghost_id, leave.clone());
      });
    }

    obj.into_widget()
  }
}

fn dispose_ghost_when_leave_done(
  window_id: WindowId, id: WidgetId, leave: Rc<Box<dyn PresenceAction>>,
) {
  let Some(wnd) = AppCtx::get_window(window_id) else { return };
  wnd.once_layout_ready(move || {
    let Some(wnd) = AppCtx::get_window(window_id) else { return };
    if id.is_dropped(wnd.tree()) {
      return;
    }
    if !leave.is_running() {
      id.dispose_subtree(wnd.tree_mut());
      return;
    }
    dispose_ghost_when_leave_done(window_id, id, leave);
  });
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
  pending: Option<(S, Box<dyn Transition>, S::Value)>,
  animate: Option<Stateful<Animate<S>>>,
}

impl<S> PresenceAction for RefCell<LeaveInner<S>>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
{
  fn fire(&self, window_id: WindowId) {
    let inner = &mut *self.borrow_mut();
    let Some((state, transition, to)) = inner.pending.take() else { return };

    let from = state.get();
    state.set(to);

    let animate = {
      let mut d = Animate::declarer();
      d.with_state(state)
        .with_from(from)
        .with_transition(transition)
        .with_window_id(window_id);
      d.finish()
    };
    animate.run();
    inner.animate = Some(animate);
  }

  fn is_running(&self) -> bool {
    self
      .borrow()
      .animate
      .as_ref()
      .is_some_and(|a| a.read().is_running())
  }
}

struct EnterInner<S: AnimateState + 'static> {
  pending: Option<(S, Box<dyn Transition>, S::Value)>,
  animate: Option<Stateful<Animate<S>>>,
}

impl<S> PresenceAction for RefCell<EnterInner<S>>
where
  S: AnimateState + 'static,
  S::Value: Clone + 'static,
{
  fn fire(&self, window_id: WindowId) {
    let inner = &mut *self.borrow_mut();
    let Some((state, transition, from)) = inner.pending.take() else { return };

    let real_value = state.get();
    state.set(from.clone());

    let animate = {
      let mut d = Animate::declarer();
      d.with_state(state)
        .with_from(from)
        .with_transition(transition)
        .with_window_id(window_id);
      d.finish()
    };
    // Set the state to the real target value so that when run() is called,
    // the animation will animate from `from` to `real_value`.
    animate.write().state.set(real_value);
    animate.run();
    inner.animate = Some(animate);
  }

  fn is_running(&self) -> bool {
    self
      .borrow()
      .animate
      .as_ref()
      .is_some_and(|a| a.read().is_running())
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
    Box::new(RefCell::new(LeaveInner {
      pending: Some((action.state, action.transition.into_box(), action.to)),
      animate: None,
    }))
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
    Box::new(RefCell::new(EnterInner {
      pending: Some((action.state, action.transition.into_box(), action.from)),
      animate: None,
    }))
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
  use super::*;
  use crate::{reset_test_env, test_helper::*, window::WindowFlags};

  #[test]
  fn leave_animation_keeps_widget_alive() {
    reset_test_env!();

    let show = Stateful::new(true);
    let c_show = show.clone_writer();
    let mounted_id = Stateful::new(None::<WidgetId>);
    let c_mounted_id = mounted_id.clone_reader();

    let w = fn_widget! {
      pipe!(*$read(show)).map(move |visible| {
        if visible {
          let mut item = @MockBox {
            margin: EdgeInsets { left: 24., top: 18., ..EdgeInsets::ZERO },
            size: Size::new(100., 30.),
            on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          };
          let opacity = item.opacity();

          let presence = Stateful::new(AnimatePresence {
            enter: None,
            leave: Some(LeaveAction {
              state: opacity,
              transition: EasingTransition {
                easing: easing::LINEAR,
                duration: Duration::from_millis(100),
              },
              to: 0.0,
            }.into()),
          });

          AnimatePresence::compose_child(presence, item.into_widget())
        } else {
          @Void {}.into_widget()
        }
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let mounted_id = c_mounted_id
      .read()
      .expect("child should be mounted before triggering leave");
    let original_pos = wnd.map_to_global(Point::zero(), mounted_id);

    // Remember how many children root had before hiding.
    let root = wnd.tree().root();
    let children_before = root.children(wnd.tree()).count();

    // Hide the widget — leave animation should fire and ghost should be
    // re-parented to root.
    *c_show.write() = false;
    wnd.draw_frame();

    // After disposal the ghost subtree is appended to root, so root should
    // have at least one more child than before.
    let children_after = root.children(wnd.tree()).count();
    assert!(
      children_after > children_before,
      "expected ghost widget appended to root (before={children_before}, after={children_after})"
    );

    let ghost_id = root
      .children(wnd.tree())
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

      let presence = Stateful::new(AnimatePresence {
        enter: Some(EnterAction {
          state: opacity,
          transition: EasingTransition {
            easing: easing::LINEAR,
            duration: Duration::from_millis(100),
          },
          from: 0.0,
        }.into()),
        leave: None,
      });

      AnimatePresence::compose_child(presence, item.into_widget())
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
        if visible {
          let mut item = @MockBox { size: Size::new(100., 30.) };
          let opacity = item.opacity();

          let presence = Stateful::new(AnimatePresence {
            enter: None,
            leave: Some(LeaveAction {
              state: opacity,
              transition: EasingTransition {
                easing: easing::LINEAR,
                duration: Duration::from_millis(100),
              },
              to: 0.0,
            }.into()),
          });

          AnimatePresence::compose_child(presence, item.into_widget())
        } else {
          @Void {}.into_widget()
        }
      })
    };

    let wnd = TestWindow::new(w, Size::new(200., 200.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();

    *c_show.write() = false;
    wnd.draw_frame();
    wnd.dispose();
  }
}
