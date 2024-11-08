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
use crate::{prelude::*, wrap_render::*};

smooth_pos_widget_impl!(SmoothPos, Point);
smooth_pos_widget_impl!(SmoothY, f32, y);
smooth_pos_widget_impl!(SmoothX, f32, x);
smooth_size_widget_impl!(SmoothSize, Size);
smooth_size_widget_impl!(SmoothHeight, f32, height);
smooth_size_widget_impl!(SmoothWidth, f32, width);

#[derive(Default)]
struct SmoothImpl<T> {
  running: bool,
  value: T,
}

impl<T: Copy + PartialEq + 'static> Stateful<SmoothImpl<T>> {
  fn transition(
    &self, transition: impl Transition + 'static,
  ) -> Stateful<Animate<impl AnimateState + 'static>>
  where
    T: Lerp,
  {
    let animate = part_writer!(&mut self.value).transition(transition);
    let this = self.clone_writer();
    watch!($animate.is_running())
      .distinct_until_changed()
      .subscribe(move |running| {
        let mut w = this.write();
        w.running = running;
        w.forget_modifies();
      });
    animate
  }
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
        let SmoothImpl { running, value } = *self.0.read();
        if running {
          clamp.min $(.$field)? = value;
          clamp.max $(.$field)? = value;
        }
        let size = host.perform_layout(clamp, ctx);
        let new_v = size $(.$field)?;
        if !running && value != new_v {
          self.0.write().value = new_v;
        }
        size
      }
    }

    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;
      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        fn_widget!{
          let modifies = this.read().0.raw_modifies();
          WrapRender::combine_child(this, child)
            .on_build(move |id, | id.dirty_on(modifies) )
        }.into_widget()
      }
    }

    impl $name {
      #[doc = "Enable the transition with the provided argument and return the animation of the transition."]
      pub fn transition(&self, transition: impl Transition + 'static)
        -> Stateful< Animate<impl AnimateState + 'static>>
      {
        self.0.transition(transition, )
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
        let smooth = self.0.clone_writer();
        if !smooth.read().running  {
          let wid = ctx.widget_id();
          let wnd = ctx.window();
          let _ = AppCtx::spawn_local(async move {
            let pos = wnd.map_to_global(Point::zero(), wid);
            smooth.write().value = pos$(.$field)?;
          });
        }

        host.perform_layout(clamp, ctx)
      }

      fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
        let SmoothImpl { running, value } = *self.0.read();
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

    impl_compose_child_for_wrap_render!($name);

    impl $name {
      #[doc = "Enable the transition with the provided argument and return the animation of the transition."]
      pub fn transition(&self, transition: impl Transition + 'static)
        -> Stateful< Animate<impl AnimateState + 'static>>
      {
        self.0.transition(transition)
      }
    }
  };
}

use smooth_pos_widget_impl;
use smooth_size_widget_impl;
