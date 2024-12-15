//! Widgets use animation to transition the layout position or size from the
//! previous layout state after each layout performed.
//!
//! While animation can work on any state of the render widget, the layout
//! information is publicly read-only data provided by the framework. Therefore,
//! providing animation for transitioning a widget's layout size and position
//! can be challenging. The `smooth_layout` module offers six widgets -
//! `SmoothX`, `SmoothY`, `SmoothWidth`, `SmoothHeight`, `SmoothPos`, and
//! `SmoothSize` - to assist in transitioning the layout information along the
//! x-axis, y-axis, width, height, both axes, and size.
//!
//! # Example
//!
//! ```rust
//! use ribir::prelude::*;
//!
//! let _smooth_move_to_center = fn_widget! {
//!
//!     // Create a smooth widget that operates on both the x-axis and y-axis.
//!     let smooth = SmoothPos::default();
//!     // Enable the transition
//!     let _animate = smooth.transition(EasingTransition {
//!        easing: easing::LinearEasing,
//!        duration: Duration::from_millis(1000),
//!     });
//!
//!     // Apply the smooth widget to the desired widget.
//!     @ $smooth {
//!         @Void {
//!             clamp: BoxClamp::fixed_size(Size::new(100., 100.)),
//!             h_align: HAlign::Center,
//!             v_align: VAlign::Center,
//!             background: Color::RED,
//!         }
//!     }
//! };
//! ```
use crate::{prelude::*, ticker::FrameMsg, window::WindowFlags, wrap_render::*};

smooth_pos_widget_impl!(SmoothPos, Point);
smooth_pos_widget_impl!(SmoothY, f32, y);
smooth_pos_widget_impl!(SmoothX, f32, x);
smooth_size_widget_impl!(SmoothSize, Size);
smooth_size_widget_impl!(SmoothHeight, f32, height);
smooth_size_widget_impl!(SmoothWidth, f32, width);

#[derive(Default, Debug)]
struct SmoothImpl<T> {
  /// Indicates whether the transition is running.
  running: bool,
  /// Indicates if a relayout is required for the widget.
  force_layout: bool,
  value: T,
}

impl<T: Copy + PartialEq + 'static> Stateful<SmoothImpl<T>> {
  fn set_running(&self, ready: bool) {
    let mut w_ref = self.write();
    w_ref.running = ready;
    w_ref.forget_modifies();
  }

  fn set_force_layout(&self, force: bool) {
    let mut w_ref = self.write();
    w_ref.force_layout = force;
    w_ref.forget_modifies();
  }

  fn transition(
    &self, transition: impl Transition + 'static,
  ) -> Stateful<Animate<impl AnimateState + 'static>>
  where
    T: Lerp,
  {
    let animate = part_writer!(&mut self.value).transition(transition);
    let this = self.clone_writer();
    watch!($animate.is_running()).subscribe(move |running| this.set_running(running));
    animate
  }
}

fn on_frame_end_once(ctx: &LayoutCtx, f: impl FnMut(FrameMsg) + 'static) {
  ctx
    .window()
    .frame_tick_stream()
    .filter(|msg| matches!(msg, FrameMsg::Finish(_)))
    .take(1)
    .subscribe(f);
}

macro_rules! smooth_size_widget_impl {
  ($name:ident, $size_ty:ty $(, $field:ident)?) => {
    #[doc = "This widget enables smooth size transitions for its child after layout.\
     See the [module-level documentation](self) for more."]
    #[derive(Default)]
    pub struct $name(Stateful<SmoothImpl<$size_ty>>);

    impl WrapRender for $name {
      fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx)
        -> Size
      {
        if !ctx.window().flags().contains(WindowFlags::ANIMATIONS) {
          return host.perform_layout(clamp, ctx);
        }

        let SmoothImpl { force_layout, value, running } = *self.0.read();
        if force_layout || !running {
          if force_layout {
            self.0.set_force_layout(false);
          }

          let size = host.perform_layout(clamp, ctx);
          let new_v = size $(.$field)?;
          if value != new_v {
            let this = self.0.clone_writer();
            // We must update the value in the next frame to ensure a
            // seamless size transition from the previous value to the new one.
            on_frame_end_once(ctx, move |_| this.write().value = new_v);
          }
        }

        clamp.min $(.$field)? = value;
        clamp.max $(.$field)? = value;
        host.perform_layout(clamp, ctx)
      }
    }

    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;
      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        let inner = this.read().0.clone_writer();
        WrapRender::combine_child(this, child).on_build(move |id| {
          // When the value changes, we mark the widget as dirty to prompt a new layout.
          // If the widget is already marked as dirty before the smooth widget marks it,
          // we don't use the smooth value. This indicates that the data of the host
          // widget has changed, requiring a smooth transition to the new layout result.
          let marker = BuildCtx::get().tree().dirty_marker();
          let h = inner.raw_modifies()
            .filter(|b| b.contains(ModifyScope::FRAMEWORK))
            .subscribe(move |_| {
              if !marker.mark(id) {
                inner.set_force_layout(true);
              }
            })
            .unsubscribe_when_dropped();
          id.attach_anonymous_data(h, BuildCtx::get_mut().tree_mut());
        })
      }
    }

    impl $name {
      #[doc = "Enable the transition with the provided argument and return the animation of the transition."]
      pub fn transition(&self, transition: impl Transition + 'static)
        -> Stateful<Animate<impl AnimateState + 'static>>
      {
        self.0.transition(transition)
      }
    }
  };
}

macro_rules! smooth_pos_widget_impl {
  ($name:ident, $size_ty:ty $(, $field:ident)?) => {
    #[doc = "This widget enables smooth position transitions for its child after layout.\
     See the [module-level documentation](self) for more."]
    #[derive(Default)]
    pub struct $name(Stateful<SmoothImpl<$size_ty>>);

    impl WrapRender for $name {
      fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
        if !ctx.window().flags().contains(WindowFlags::ANIMATIONS) {
          return host.perform_layout(clamp, ctx);
        }

        let SmoothImpl { force_layout, running,.. } = *self.0.read();

        if force_layout || !running {
          let smooth = self.0.clone_writer();

          if !running {
            // As the animation begins in the next frame, we manually mark it as
            // running to ensure that this frame displays the smooth value instead
            // of the actual value, maintaining a smooth animation.
            smooth.set_running(true);
          }
          if force_layout {
            smooth.set_force_layout(false);
          }

          let wid = ctx.widget_id();
          let wnd = ctx.window();
          // We need to wait until the end of this frame to determine
          // the position of the widget.
          on_frame_end_once(ctx, move |_| {
            let pos = wnd.map_to_global(Point::zero(), wid);
            if smooth.read().value != pos $(.$field)? {
              smooth.write().value = pos $(.$field)?;
            } else if !running {
              // If the position has not changed, indicating that the animation
              // has not started, we revert the running state.
              smooth.set_running(false);
            }
          });
        }

        host.perform_layout(clamp, ctx)
      }

      fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
        if !ctx.window().flags().contains(WindowFlags::ANIMATIONS) {
          return host.paint(ctx);
        }

        let SmoothImpl { running, value,.. } = *self.0.read();
        if running {
          let pos = ctx.map_to_global(Point::zero());
          #[allow(unused_assignments)]
          let mut expect = pos;
          expect $(.$field)? = value;

          let offset = expect - pos;
          ctx.painter().translate(offset.x, offset.y);
        }
        host.paint(ctx);
      }
    }

    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;
      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        let inner = this.read().0.clone_writer();
        WrapRender::combine_child(this, child).on_build(move |id| {
          // A smooth transition is activated when the value changes. However, if
          // the widget is already marked as dirty, we will cancel the smooth
          // transition because it indicates that a new layout has been triggered.
          let marker = BuildCtx::get().tree().dirty_marker();
          let h = inner.raw_modifies()
            .filter(|b| b.contains(ModifyScope::FRAMEWORK))
            .subscribe(move |_| {
              if marker.is_dirty(id) {
                inner.set_force_layout(true)
              }
            })
            .unsubscribe_when_dropped();
          id.attach_anonymous_data(h, BuildCtx::get_mut().tree_mut());
        })
      }
    }


    impl $name {
      #[doc = "Enable the transition with the provided argument and return the animation of the transition."]
      pub fn transition(&self, transition: impl Transition + 'static)
         -> Stateful<Animate<impl AnimateState + 'static>>
      {
        self.0.transition(transition)
      }
    }
  };
}

use smooth_pos_widget_impl;
use smooth_size_widget_impl;
