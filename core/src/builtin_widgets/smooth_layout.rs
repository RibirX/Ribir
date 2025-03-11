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
//! These widgets have two fields in their declaration: `transition` and
//! `init_value`.
//!
//! - **transition**: specifies the animated transition for the layout
//!   information.
//! - **init_value**: specifies the initial value of the layout information
//!   before the first layout.
//!
//! # Example
//!
//! ```rust
//! use ribir::prelude::*;
//!
//! let _smooth_move_to_center = smooth_pos! {
//!   transition: EasingTransition {
//!      easing: easing::LinearEasing,
//!      duration: Duration::from_millis(1000),
//!   },
//!   init_value: Anchor::right(Measure::Percent(0.1)),
//!   @Void {
//!     clamp: BoxClamp::fixed_size(Size::new(100., 100.)),
//!     h_align: HAlign::Center,
//!     v_align: VAlign::Center,
//!     background: Color::RED,
//!   }
//! };
//! ```
//!
//! In the provided example, an `init_value` is given, causing the widget to
//! move in from the right, positioned 10 percent from the right edge of the
//! window. If no `init_value` is provided, the first layout will not be
//! animated.
use crate::{prelude::*, window::WindowFlags, wrap_render::*};

/// This widget enables smooth position transitions for its declare child
/// between layout. See the [module-level documentation](self) for more.
pub struct SmoothPos(Stateful<SmoothImpl<Anchor, Point>>);

/// This widget enables smooth transitions for its declare child's x-axis
/// between layout.
pub struct SmoothX(Stateful<SmoothImpl<HAnchor, f32>>);

/// This widget enables smooth transitions for its declare child's y-axis
/// between layout.
pub struct SmoothY(Stateful<SmoothImpl<VAnchor, f32>>);

/// This widget enables smooth transitions for its declare child's size
/// between layout. See the [module-level documentation](self) for more.
pub struct SmoothSize(Stateful<SmoothImpl<Size<Measure>, Size>>);

/// This widget enables smooth transitions for its declare child's width
/// between layout. See the [module-level documentation](self) for more.
pub struct SmoothWidth(Stateful<SmoothImpl<Measure, f32>>);

/// This widget enables smooth transitions for its declare child's height
/// between layout. See the [module-level documentation](self) for more.
pub struct SmoothHeight(Stateful<SmoothImpl<Measure, f32>>);

/// Creates a function widget that utilizes `SmoothPos` as its root widget.
#[macro_export]
macro_rules! smooth_pos {
  ($($t: tt)*) => { fn_widget! { @SmoothPos { $($t)* } } };
}

/// Creates a function widget that utilizes `SmoothX` as its root widget.
#[macro_export]
macro_rules! smooth_x {
  ($($t: tt)*) => { fn_widget! { @SmoothX { $($t)* } } };
}

/// Creates a function widget that utilizes `SmoothY` as its root widget.
#[macro_export]
macro_rules! smooth_y {
  ($($t: tt)*) => { fn_widget! { @SmoothY { $($t)* } } };
}

/// Creates a function widget that utilizes `SmoothSize` as its root widget.
#[macro_export]
macro_rules! smooth_size {
  ($($t: tt)*) => { fn_widget! { @SmoothSize { $($t)* } } };
}

/// Creates a function widget that utilizes `SmoothWidth` as its root widget.
#[macro_export]
macro_rules! smooth_width {
  ($($t: tt)*) => { fn_widget! { @SmoothWidth { $($t)* } } };
}

/// Creates a function widget that utilizes `SmoothHeight` as its root widget.
#[macro_export]
macro_rules! smooth_height {
  ($($t: tt)*) => { fn_widget! { @SmoothHeight { $($t)* } } };
}

smooth_pos_widget_impl!(SmoothPos);
smooth_pos_widget_impl!(SmoothY);
smooth_pos_widget_impl!(SmoothX);
smooth_size_widget_impl!(SmoothSize);
smooth_size_widget_impl!(SmoothHeight);
smooth_size_widget_impl!(SmoothWidth);

impl_smooth_layout_declare!(SmoothPos, Anchor);
impl_smooth_layout_declare!(SmoothY, VAnchor);
impl_smooth_layout_declare!(SmoothX, HAnchor);
impl_smooth_layout_declare!(SmoothSize, Size<Measure>);
impl_smooth_layout_declare!(SmoothWidth, Measure);
impl_smooth_layout_declare!(SmoothHeight, Measure);

#[derive(Debug, Clone, Copy, PartialEq)]
enum SmoothValue<I, T> {
  Init(Option<I>),
  Value(T),
}

#[derive(Default, Debug)]
struct SmoothImpl<I, T> {
  /// Indicates whether the transition is running.
  running: bool,
  /// Indicates if a relayout is required for the widget.
  force_layout: bool,
  value: SmoothValue<I, T>,
}

impl<I, T> Stateful<SmoothImpl<I, T>>
where
  Self: 'static,
{
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

  fn transition(&self, transition: impl Transition + 'static)
  where
    SmoothValue<I, T>: Lerp + PartialEq + Clone + std::fmt::Debug + 'static,
  {
    let animate = part_writer!(&mut self.value).transition(transition);
    let this = self.clone_writer();
    watch!($animate.is_running()).subscribe(move |running| this.set_running(running));
  }
}

macro_rules! smooth_size_widget_impl {
  ($name:ident) => {
    impl WrapRender for $name {
      fn perform_layout(
        &self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx,
      ) -> Size {
        if !ctx
          .window()
          .flags()
          .contains(WindowFlags::ANIMATIONS)
        {
          return host.perform_layout(clamp, ctx);
        }

        self.switch_init_to_value(clamp.max);

        let SmoothImpl { force_layout, running, .. } = *self.0.read();

        if force_layout || !running {
          if force_layout {
            self.0.set_force_layout(false);
          }

          let size = host.perform_layout(clamp, ctx);
          // We need to modify the real size to trigger the animation, but we will
          // delay this action until the next frame begins to avoid disturbing the
          // layout and animation logic.
          let this = $name(self.0.clone_writer());
          AppCtx::once_next_frame(move |_| this.set_size(size))
        }

        self.clamp_layout_clamp(&mut clamp);
        host.perform_layout(clamp, ctx)
      }

      fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase {
        let dirty = host.dirty_phase();
        if dirty != DirtyPhase::LayoutSubtree { DirtyPhase::Layout } else { dirty }
      }
    }

    impl_compose_child!($name, DirtyPhase::Layout);
  };
}

macro_rules! smooth_pos_widget_impl {
  ($name:ident) => {
    impl WrapRender for $name {
      fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
        if !ctx
          .window()
          .flags()
          .contains(WindowFlags::ANIMATIONS)
        {
          return host.perform_layout(clamp, ctx);
        }

        let SmoothImpl { force_layout, running, .. } = *self.0.read();

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

          // We need to wait until the end of this frame to determine
          // the position of the widget.
          let wid = ctx.widget_id();
          let wnd = ctx.window();
          let smooth = $name(smooth);
          AppCtx::once_next_frame(move |_| {
            let pos = wnd.map_to_global(Point::zero(), wid);
            if !smooth.set_pos(pos) && !running {
              // If the position has not changed, indicating that the animation
              // has not started, we revert the running state.
              smooth.0.set_running(false);
            }
          });
        }

        let size = host.perform_layout(clamp, ctx);
        self.switch_init_to_value(size, clamp.max);
        size
      }

      fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
        if !ctx
          .window()
          .flags()
          .contains(WindowFlags::ANIMATIONS)
        {
          return host.paint(ctx);
        }

        let SmoothImpl { running, ref value, .. } = *self.0.read();
        if running {
          let pos = ctx.map_to_global(Point::zero());
          let offset = value.get_pos(pos) - pos;
          ctx.painter().translate(offset.x, offset.y);
        }
        host.paint(ctx);
      }

      fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase {
        let dirty = host.dirty_phase();
        if dirty != DirtyPhase::LayoutSubtree { DirtyPhase::Layout } else { dirty }
      }
    }

    impl_compose_child!($name, DirtyPhase::Paint);
  };
}

macro_rules! impl_compose_child {
  ($name:ty, $dirty:expr) => {
    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;

      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        let track = TrackWidgetId::default();
        let id = track.track_id();
        let ctx = BuildCtx::get();
        let marker = ctx.tree().dirty_marker();
        let window = ctx.window();

        // The animation utilizes the smooth value for a seamless visual transition.
        // Throughout the animation, we must monitor if the widget has been altered by
        // external factors. If any modifications occur, we must initiate a forced
        // layout to ensure the animation transitions to a new and accurate layout
        // result.
        let inner = this.read().0.clone_writer();
        let h = inner
          .raw_modifies()
          .filter(|b| b.contains(ModifyScope::FRAMEWORK))
          .subscribe(move |_| {
            let inner = inner.clone_writer();
            let marker = marker.clone();
            let id = id.get().unwrap();
            window.once_before_layout(move || {
              if marker.is_dirty(id) {
                inner.set_force_layout(true)
              }
              marker.mark(id, $dirty);
            })
          })
          .unsubscribe_when_dropped();
        let child = track
          .with_child(child)
          .into_widget()
          .attach_anonymous_data(h);

        WrapRender::combine_child(this, child, $dirty)
      }
    }
  };
}

macro_rules! impl_smooth_layout_declare {
  ($name:ty, $init_ty:ty) => {
    paste::paste! {
      #[derive(Default)]
      pub struct [<$name Declarer>] {
        transition: Option<Box<dyn Transition>>,
        init_value: Option<$init_ty>,
      }

      pub trait [<$name DeclareExtend>] {
        /// How to transition the layout value.
        fn transition(self, transition: impl Transition + 'static) -> Self;
        /// The initial value that the first layout will transition from.
        fn init_value(self, init_value: impl Into<$init_ty>) -> Self;
      }

      impl [<$name DeclareExtend>] for FatObj<[<$name Declarer>]> {
        fn transition(mut self, transition: impl Transition + 'static) -> Self {
          self.transition = Some(Box::new(transition));
          self
        }

        fn init_value(mut self, init_value: impl Into<$init_ty>) -> Self {
          self.init_value = Some(init_value.into());
          self
        }
      }

      impl Declare for $name {
        type Builder = FatObj<[<$name Declarer>]>;

        #[inline]
        fn declarer() -> Self::Builder {
          FatObj::new([<$name Declarer>]::default())
        }
      }

      impl FatDeclarerExtend for [<$name Declarer>] {
        type Target = $name;

        fn finish(mut this: FatObj<Self>) -> FatObj<Self::Target> {
            let transition = this.transition.take().unwrap_or_else(|| {
            Box::new(EasingTransition {
              easing: easing::LinearEasing,
              duration: Duration::from_millis(200),
            })
          });
          let value = SmoothValue::Init(this.init_value.take());
          let w = $name(Stateful::new(SmoothImpl { running: false, force_layout: false, value }));
          w.0.transition(transition);

          this.map(|_| w)
        }
      }
    }
  };
}

use impl_compose_child;
use impl_smooth_layout_declare;
use smooth_pos_widget_impl;
use smooth_size_widget_impl;

impl<I, T: Default> Default for SmoothValue<I, T> {
  fn default() -> Self { Self::Value(T::default()) }
}

impl<I, T: Lerp> Lerp for SmoothValue<I, T>
where
  Self: Clone,
{
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    match (self, to) {
      (SmoothValue::Value(from), SmoothValue::Value(to)) => {
        SmoothValue::Value(from.lerp(to, factor))
      }
      // The initial value is not interpolated.
      _ => to.clone(),
    }
  }
}

impl<I, T: Copy> SmoothValue<I, T> {
  fn get(&self) -> Option<T> {
    match self {
      SmoothValue::Value(v) => Some(*v),
      SmoothValue::Init(_) => None,
    }
  }
}

impl SmoothValue<Anchor, Point> {
  fn get_pos(&self, default: Point) -> Point {
    match self {
      SmoothValue::Value(v) => *v,
      SmoothValue::Init(_) => default,
    }
  }
}

impl SmoothValue<HAnchor, f32> {
  fn get_pos(&self, default: Point) -> Point {
    match self {
      SmoothValue::Value(v) => Point::new(*v, default.y),
      SmoothValue::Init(_) => default,
    }
  }
}

impl SmoothValue<VAnchor, f32> {
  fn get_pos(&self, default: Point) -> Point {
    match self {
      SmoothValue::Value(v) => Point::new(default.x, *v),
      SmoothValue::Init(_) => default,
    }
  }
}

impl SmoothPos {
  fn switch_init_to_value(&self, size: Size, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let pos = v.into_pixel(size, max_clamp);
    self.0.write().value = SmoothValue::Value(pos);
  }

  fn set_pos(&self, pos: Point) -> bool {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == pos);
    if !same {
      self.0.write().value = SmoothValue::Value(pos);
    }
    !same
  }
}

impl SmoothX {
  fn switch_init_to_value(&self, size: Size, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let x = v.into_pixel(size.width, max_clamp.width);
    self.0.write().value = SmoothValue::Value(x);
  }

  fn set_pos(&self, pos: Point) -> bool {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == pos.x);
    if !same {
      self.0.write().value = SmoothValue::Value(pos.x);
    }
    !same
  }
}

impl SmoothY {
  fn switch_init_to_value(&self, size: Size, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let y = v.into_pixel(size.height, max_clamp.height);
    self.0.write().value = SmoothValue::Value(y);
  }

  fn set_pos(&self, pos: Point) -> bool {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == pos.y);
    if !same {
      self.0.write().value = SmoothValue::Value(pos.y);
    }
    !same
  }
}

impl SmoothSize {
  fn switch_init_to_value(&self, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let value =
      Size::new(v.width.into_pixel(max_clamp.width), v.height.into_pixel(max_clamp.height));
    self.0.write().value = SmoothValue::Value(value);
  }

  fn set_size(&self, size: Size) {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == size);
    if !same {
      self.0.write().value = SmoothValue::Value(size);
    }
  }

  fn clamp_layout_clamp(&self, clamp: &mut BoxClamp) {
    if let Some(value) = self.0.read().value.get() {
      clamp.max = value;
      clamp.min = value;
    }
  }
}

impl SmoothWidth {
  fn switch_init_to_value(&self, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let width = v.into_pixel(max_clamp.width);
    self.0.write().value = SmoothValue::Value(width);
  }

  fn set_size(&self, size: Size) {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == size.width);
    if !same {
      self.0.write().value = SmoothValue::Value(size.width);
    }
  }

  fn clamp_layout_clamp(&self, clamp: &mut BoxClamp) {
    if let Some(value) = self.0.read().value.get() {
      clamp.max.width = value;
      clamp.min.width = value;
    }
  }
}

impl SmoothHeight {
  fn switch_init_to_value(&self, max_clamp: Size) {
    let SmoothValue::Init(Some(v)) = self.0.read().value else { return };
    let height = v.into_pixel(max_clamp.height);
    self.0.write().value = SmoothValue::Value(height);
  }

  fn set_size(&self, size: Size) {
    let same = matches!(self.0.read().value, SmoothValue::Value(a) if a == size.height);
    if !same {
      self.0.write().value = SmoothValue::Value(size.height);
    }
  }

  fn clamp_layout_clamp(&self, clamp: &mut BoxClamp) {
    if let Some(value) = self.0.read().value.get() {
      clamp.max.height = value;
      clamp.min.height = value;
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir::{
    core::{reset_test_env, test_helper::*, window::WindowFlags},
    prelude::{easing::LinearEasing, *},
  };
  use ribir_dev_helper::*;

  const TEST_TRANS: EasingTransition<LinearEasing> =
    EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(200) };

  fn center_red_block_10_x_10() -> Widget<'static> {
    container! {
      background: Color::RED,
      size: Size::new(10., 10.),
      h_align: Align::Center,
      v_align: Align::Center
    }
    .into_widget()
  }

  fn red_block_10_x_10() -> Widget<'static> {
    container! {
      background: Color::RED,
      size: Size::new(10., 10.),
    }
    .into_widget()
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_pos() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(stack! {
        // If no initial value is provided, the widget should start at its real place.
        @SmoothPos {
          transition: TEST_TRANS,
          @center_red_block_10_x_10()
        }
        // Begin at the top left.
        @SmoothPos {
          transition: TEST_TRANS,
          init_value: Anchor::left_top(5., Measure::Percent(0.1)),
          @center_red_block_10_x_10()
        }
        // Begin at the bottom right.
        @SmoothPos {
          transition: TEST_TRANS,
          init_value: Anchor::right_bottom(Measure::Percent(0.1), 5.),
          @center_red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_pos"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_x() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(self::column! {
        clamp: BoxClamp::default().with_max_width(100.),
        align_items: Align::Center,
        // If no initial value is provided, the widget should start at its real place.
        @SmoothX {
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        // Begin at the left 10 percent.
        @SmoothX {
          transition: TEST_TRANS,
          init_value: Measure::Percent(0.1),
          @red_block_10_x_10()
        }
        // Begin at the right.
        @SmoothX {
          transition: TEST_TRANS,
          init_value: HAnchor::Right(0f32.into()),
          @red_block_10_x_10()
        }
        @SizedBox { size: Size::new(100., 10.) }
      })
      .with_wnd_size(Size::new(100., 30.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_x"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_y() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(self::row! {
        clamp: BoxClamp::default().with_max_height(100.),
        align_items: Align::Center,
        // If no initial value is provided, the widget should start at its real place.
        @SmoothY {
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        // Begin at the top 10 percent.
        @SmoothY {
          transition: TEST_TRANS,
          init_value: Measure::Percent(0.1),
          @red_block_10_x_10()
        }
        // Begin at the bottom.
        @SmoothY {
          transition: TEST_TRANS,
          init_value: VAnchor::Bottom(0f32.into()),
          @red_block_10_x_10()
        }
        @SizedBox { size: Size::new(10., 100.) }
      })
      .with_wnd_size(Size::new(30., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_y"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_size() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(crate::smooth_size! {
        transition: TEST_TRANS,
        init_value: Size::splat(Measure::Percent(0.5)),
        @center_red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_size_from_50p"
    );

    assert_widget_eq_image!(
      WidgetTester::new(crate::smooth_size! {
        transition: TEST_TRANS,
        init_value: Size::splat(5f32.into()),
        @center_red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_size_from_5"
    );

    assert_widget_eq_image!(
      WidgetTester::new(crate::smooth_size! {
        transition: TEST_TRANS,
        @center_red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_size_from_real"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_width() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(self::column! {
        item_gap: 2.,
        // If no initial value is provided, begin at its real place.
        @SmoothWidth {
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        // Begin at 50 percent.
        @SmoothWidth {
          transition: TEST_TRANS,
          init_value: Measure::Percent(0.5),
          @red_block_10_x_10()
        }
        // Begin at 5 px.
        @SmoothWidth {
          transition: TEST_TRANS,
          init_value: 5.,
          @red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(100., 40.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_width"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_height() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(self::row! {
        item_gap: 2.,
        // If no initial value is provided, begin at its real place.
        @SmoothHeight {
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        // Begin at 50 percent.
        @SmoothHeight {
          transition: TEST_TRANS,
          init_value: Measure::Percent(0.5),
          @red_block_10_x_10()
        }
        // Begin at 5 px.
        @SmoothHeight {
          transition: TEST_TRANS,
          init_value: 5.,
          @red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(40., 100.))
      .on_initd(|wnd| wnd.set_flags(WindowFlags::ANIMATIONS)),
      "smooth_height"
    );
  }
}
