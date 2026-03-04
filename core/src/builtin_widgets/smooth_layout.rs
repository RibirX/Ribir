//! Smooth layout widgets that animate position and/or size transitions between
//! layout updates.
//!
//! # Overview
//!
//! [`SmoothLayout`] is a single wrapper widget that intercepts the layout
//! pipeline and interpolates geometry — position and/or size — from a *from*
//! value toward a *target* value on each frame. The animation runs entirely
//! inside the layout/paint pipeline: no reactive subscription chain is needed,
//! and dirty marking is self-scheduled through `paint()`.
//!
//! Axis selection is controlled by [`SmoothAxes`]. The default is
//! [`SmoothAxes::ALL`] (both position and size). Use bit-flag combinations to
//! restrict animation to a subset of axes.
//!
//! # Position smoothing (`SmoothAxes::POS / X / Y`)
//!
//! For position animation, **bind the dynamic `x`/`y` on the child widget**,
//! not on `SmoothLayout` itself. `SmoothLayout` reads the position that the
//! parent assigns (via `adjust_position`) and interpolates it frame-by-frame.
//!
//! ```rust,ignore
//! @SmoothLayout {
//!   axes: SmoothAxes::POS,
//!   // Optional: where to start from on first appearance
//!   init_pos: Anchor::left_top(0., 100.),
//!   @MyWidget {
//!     // Moving this x drives the smooth animation
//!     x: pipe!(AnchorX::left().offset(*$read(offset))),
//!   }
//! }
//! ```
//!
//! # Size smoothing (`SmoothAxes::SIZE / WIDTH / HEIGHT`)
//!
//! For size animation, the behaviour visible to the rest of the layout is
//! governed by [`SizeMode`]:
//!
//! | [`SizeMode`]               | Effect                                             |
//! |----------------------------|----------------------------------------------------|
//! | [`SizeMode::Visual`]       | Layout reports *target* size; animation is visual only. |
//! | [`SizeMode::Layout`]       | Layout reports *animated* size and relayouts the child. |
//!
//! The visual effect during animation is set by [`ContentMotion`]:
//!
//! | [`ContentMotion`]           | Effect                                             |
//! |-----------------------------|----------------------------------------------------|
//! | [`ContentMotion::ClipReveal`] | Clips paint output to animated size (default).   |
//! | [`ContentMotion::Scale`]      | Scales content from basis size to animated size. |
//!
//! ```rust,ignore
//! // Reveal a widget by expanding from 0 width
//! @SmoothLayout {
//!   axes: SmoothAxes::WIDTH,
//!   init_width: 0.,
//!   @MyWidget {}
//! }
//!
//! // Smooth size transition with scale effect
//! @SmoothLayout {
//!   axes: SmoothAxes::SIZE,
//!   content_motion: ContentMotion::Scale,
//!   init_size: Size::splat(0f32.into()),
//!   @MyWidget {}
//! }
//! ```
//!
//! # Initial value
//!
//! On *first appearance* you can specify where to animate from via:
//! - `init_pos` / `init_x` / `init_y` — initial position ([`Anchor`]).
//! - `init_size` / `init_width` / `init_height` — initial size ([`Measure`],
//!   accepts pixels or percentages of the containing box).
//!
//! If no init value is provided the widget appears at its target immediately
//! (no entry animation).
//!
//! # Transition
//!
//! The default transition is a 200 ms linear ease. Override it with any
//! [`Transition`] implementor via `transition`:
//!
//! ```rust,ignore
//! @SmoothLayout {
//!   transition: EasingTransition {
//!     easing: easing::EASE_OUT,
//!     duration: Duration::from_millis(350),
//!   },
//!   @MyWidget {}
//! }
//! ```
//!
//! # Macro shorthand
//!
//! Use [`smooth_layout!`] as a shorthand for
//! `fn_widget! { @SmoothLayout { … } }`.
use std::cell::Cell;

use bitflags::bitflags;

use crate::{prelude::*, window::WindowFlags, wrap_render::*};

/// Controls how animated size changes interact with layout.
///
/// This setting only applies when size axes (`W`/`H`) are active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeMode {
  /// Keep the reported layout size at the *target* (final) size at all times.
  /// The animation is purely visual: neighbours are never reflowed, and the
  /// child continues to lay out at target size.
  Visual,
  /// Report the *animated* size to the parent and re-layout the child at the
  /// animated size on every frame.
  ///
  /// This is the default.
  #[default]
  Layout,
}

/// Controls the visual effect applied to content during size animation.
///
/// This only has an effect while a size animation is actively running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentMotion {
  /// Clip the painted output to the current animated size, progressively
  /// revealing (or concealing) content. Hit-testing is also restricted to the
  /// animated bounds.
  ///
  /// This is the default.
  #[default]
  ClipReveal,
  /// Apply a scale transform so that the content appears to grow or shrink
  /// smoothly. The transform origin is the top-left corner of the widget.
  /// Hit-testing maps pointer positions through the inverse transform.
  Scale,
}

bitflags! {
  /// Selects which geometry axes participate in smooth interpolation.
  ///
  /// Flags can be combined freely. The default builder value is
  /// [`SmoothAxes::ALL`].
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
  pub struct SmoothAxes: u8 {
    /// Horizontal position (X offset).
    const X = 1 << 0;
    /// Vertical position (Y offset).
    const Y = 1 << 1;
    /// Width.
    const W = 1 << 2;
    /// Height.
    const H = 1 << 3;

    /// Both position axes (`X | Y`).
    const POS = Self::X.bits() | Self::Y.bits();
    /// Both size axes (`W | H`).
    const SIZE = Self::W.bits() | Self::H.bits();
    /// Width only (alias for `W`).
    const WIDTH = Self::W.bits();
    /// Height only (alias for `H`).
    const HEIGHT = Self::H.bits();
    /// All four axes.
    const ALL = Self::X.bits() | Self::Y.bits() | Self::W.bits() | Self::H.bits();
  }
}

/// A wrapper widget that smoothly animates position and/or size transitions
/// between layout updates.
///
/// `SmoothLayout` intercepts the layout pipeline on each frame, interpolates
/// geometry (origin and/or size) from a *from* value toward a *target* value
/// using a configurable [`Transition`], and reports the interpolated result
/// back to the layout system. The animation is entirely self-driven — no
/// reactive subscription chain is required. Dirty marking is self-scheduled via
/// `paint()`: after each paint the widget checks whether a tween is still
/// running and, if so, marks itself dirty so the framework automatically
/// schedules the next frame.
///
/// See the [module-level documentation](self) for a full usage guide.
#[declare(stateless)]
pub struct SmoothLayout {
  #[declare(default = SmoothAxes::ALL)]
  axes: SmoothAxes,
  #[declare(default)]
  size_mode: SizeMode,
  #[declare(default)]
  content_motion: ContentMotion,
  #[declare(custom, default = default_transition())]
  transition: Rc<Box<dyn Transition>>,
  #[declare(default)]
  init_pos: Anchor,
  #[declare(custom, default)]
  init_width: Option<Measure>,
  #[declare(custom, default)]
  init_height: Option<Measure>,
  #[declare(skip, default = Rc::new(Cell::new(SmoothRuntime::default())))]
  runtime: Rc<Cell<SmoothRuntime>>,
}

#[derive(Clone, Copy)]
struct ActiveTween {
  from: Rect,
  to: Rect,
  started_at: Instant,
}

#[derive(Clone, Copy, Default)]
struct SmoothRuntime {
  /// Latest target geometry. `measure` writes the size; `adjust_position`
  /// writes the origin.
  target: Rect,
  /// True after the first full target has been committed.
  initialized: bool,
  /// Active animation timeline, if any.
  active: Option<ActiveTween>,
}

/// Interpolate `from` → `to` per-axis, leaving non-selected axes at `to`.
fn lerp_rect_axes(from: &Rect, to: &Rect, factor: f32, axes: SmoothAxes) -> Rect {
  let mut out = *to;
  if axes.contains(SmoothAxes::X) {
    out.origin.x = from.origin.x.lerp(&to.origin.x, factor);
  }
  if axes.contains(SmoothAxes::Y) {
    out.origin.y = from.origin.y.lerp(&to.origin.y, factor);
  }
  if axes.contains(SmoothAxes::W) {
    out.size.width = from.size.width.lerp(&to.size.width, factor);
  }
  if axes.contains(SmoothAxes::H) {
    out.size.height = from.size.height.lerp(&to.size.height, factor);
  }
  out
}

impl SmoothLayout {
  fn has_pos_axes(&self) -> bool { self.axes.intersects(SmoothAxes::POS) }

  fn has_size_axes(&self) -> bool { self.axes.intersects(SmoothAxes::SIZE) }

  fn target_size(&self) -> Size { self.runtime.get().target.size }

  fn sample_presented_rect(&self, runtime: &mut SmoothRuntime, now: Instant) -> Rect {
    let Some(active) = runtime.active else {
      return runtime.target;
    };

    let progress = self
      .transition
      .rate_of_change(now - active.started_at);
    if progress.is_finish() {
      runtime.active = None;
      active.to
    } else {
      lerp_rect_axes(&active.from, &active.to, progress.value(), self.axes)
    }
  }

  fn presented_rect(&self, now: Instant) -> Rect {
    let mut runtime = self.runtime.get();
    let rect = self.sample_presented_rect(&mut runtime, now);
    self.runtime.set(runtime);
    rect
  }

  fn is_animating(&self, now: Instant) -> bool {
    // Advance animation state (may clear `active` when the tween finishes).
    self.presented_rect(now);
    self.runtime.get().active.is_some()
  }

  fn animated_size(&self, now: Instant) -> Size { self.presented_rect(now).size }

  fn scale_factor(&self, now: Instant) -> Vector {
    let basis = self.target_size();
    let animated = self.animated_size(now);

    let sx = if basis.width > 0. { animated.width / basis.width } else { 1. };
    let sy = if basis.height > 0. { animated.height / basis.height } else { 1. };
    Vector::new(sx, sy)
  }

  fn required_dirty_phase(&self) -> DirtyPhase {
    if !self.has_size_axes() && self.has_pos_axes() {
      DirtyPhase::Position
    } else if self.has_size_axes() && self.size_mode == SizeMode::Visual && !self.has_pos_axes() {
      DirtyPhase::Paint
    } else {
      DirtyPhase::Layout
    }
  }

  fn is_content_motion_active(&self, animations_on: bool, now: Instant) -> bool {
    self.has_size_axes()
      && self.size_mode == SizeMode::Visual
      && animations_on
      && self.is_animating(now)
  }

  fn resolve_init_rect(&self, target: Rect, clamp: BoxClamp) -> Option<Rect> {
    let mut from = target;

    if self.has_size_axes() {
      if self.axes.contains(SmoothAxes::W)
        && let Some(v) = self.init_width
      {
        from.size.width = v.into_pixel(clamp.max.width);
      }
      if self.axes.contains(SmoothAxes::H)
        && let Some(v) = self.init_height
      {
        from.size.height = v.into_pixel(clamp.max.height);
      }
    }

    let max =
      Size::new(clamp.container_width(from.size.width), clamp.container_height(from.size.height));
    if self.axes.contains(SmoothAxes::X)
      && let Some(anchor) = &self.init_pos.x
    {
      from.origin.x = anchor.calculate(max.width, from.size.width);
    }
    if self.axes.contains(SmoothAxes::Y)
      && let Some(anchor) = &self.init_pos.y
    {
      from.origin.y = anchor.calculate(max.height, from.size.height);
    }

    (from != target).then_some(from)
  }

  fn start_tween(
    &self, runtime: &mut SmoothRuntime, from: Rect, to: Rect, started_at: Instant,
    animations_on: bool,
  ) {
    if !animations_on || self.axes.is_empty() || from == to {
      runtime.active = None;
    } else {
      runtime.active = Some(ActiveTween { from, to, started_at });
    }
  }

  fn try_commit_initial_target(&self, clamp: BoxClamp, window: &Rc<Window>) {
    let mut runtime = self.runtime.get();
    if runtime.initialized {
      return;
    }

    let now = Instant::now();
    let target = runtime.target;
    let from = self
      .resolve_init_rect(target, clamp)
      .unwrap_or(target);
    runtime.initialized = true;
    self.start_tween(&mut runtime, from, target, now, animations_enabled(window));
    self.runtime.set(runtime);
  }

  fn retarget_to(&self, new_target: Rect, window: &Rc<Window>) {
    let mut runtime = self.runtime.get();
    let now = Instant::now();
    let current = self.sample_presented_rect(&mut runtime, now);
    if runtime.target == new_target {
      self.runtime.set(runtime);
      return;
    }

    runtime.target = new_target;
    self.start_tween(&mut runtime, current, new_target, now, animations_enabled(window));
    self.runtime.set(runtime);
  }
}

impl Drop for SmoothLayout {
  fn drop(&mut self) {
    let mut runtime = self.runtime.get();
    runtime.active = None;
    self.runtime.set(runtime);
  }
}

fn point_in_size(pos: Point, size: Size) -> bool {
  pos.x >= 0. && pos.y >= 0. && pos.x <= size.width && pos.y <= size.height
}

fn animations_enabled(window: &Rc<Window>) -> bool {
  window.flags().contains(WindowFlags::ANIMATIONS)
}

impl<'c> ComposeChild<'c> for SmoothLayout {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for SmoothLayout {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    let window = ctx.window();
    let target_size = host.measure(clamp, ctx);
    let mut runtime = self.runtime.get();
    if runtime.initialized {
      let mut new_target = runtime.target;
      self.runtime.set(runtime);
      new_target.size = target_size;
      self.retarget_to(new_target, &window);
    } else {
      runtime.target.size = target_size;
      self.runtime.set(runtime);
      // No pos axes: commit now. With pos axes: wait for adjust_position
      // (always follows measure) to supply the full origin first.
      if !self.has_pos_axes() {
        self.try_commit_initial_target(clamp, &window);
      }
    }

    let now = Instant::now();
    let layout_size = if self.has_size_axes() && self.size_mode == SizeMode::Layout {
      self.animated_size(now)
    } else {
      self.target_size()
    };

    if self.has_size_axes() && self.size_mode == SizeMode::Layout && layout_size != target_size {
      let mut smooth_clamp = clamp;
      if self.axes.contains(SmoothAxes::W) {
        smooth_clamp.min.width = layout_size.width;
        smooth_clamp.max.width = layout_size.width;
      }
      if self.axes.contains(SmoothAxes::H) {
        smooth_clamp.min.height = layout_size.height;
        smooth_clamp.max.height = layout_size.height;
      }
      host.measure(smooth_clamp, ctx)
    } else {
      layout_size
    }
  }

  fn place_children(&self, size: Size, host: &dyn Render, ctx: &mut PlaceCtx) {
    if self.has_size_axes() && self.size_mode == SizeMode::Visual {
      host.place_children(self.target_size(), ctx)
    } else {
      host.place_children(size, ctx)
    }
  }

  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    let target_pos = host.adjust_position(pos, ctx);
    let window = ctx.window();
    let clamp = ctx.clamp();
    let mut runtime = self.runtime.get();
    if runtime.initialized {
      let mut new_target = runtime.target;
      self.runtime.set(runtime);
      new_target.origin = target_pos;
      self.retarget_to(new_target, &window);
    } else {
      runtime.target.origin = target_pos;
      self.runtime.set(runtime);
      self.try_commit_initial_target(clamp, &window);
    }

    self.presented_rect(Instant::now()).origin
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let now = Instant::now();

    // Sample BEFORE is_content_motion_active, which advances the tween via
    // sample_presented_rect and may clear `active`. Sampling here ensures the
    // final dirty mark is not missed on the last frame of an animation.
    let was_active = self.runtime.get().active.is_some();

    if self.is_content_motion_active(animations_enabled(&ctx.window()), now) {
      match self.content_motion {
        ContentMotion::ClipReveal => {
          let rect = Rect::from_size(self.animated_size(now));
          ctx.box_painter().clip(Path::rect(&rect).into());
        }
        ContentMotion::Scale => {
          let scale = self.scale_factor(now);
          if scale != Vector::one() {
            ctx.painter().scale(scale.x, scale.y);
          }
        }
      }
    }

    host.paint(ctx);

    // Drive the frame loop. Covers both cases: tween still running (active
    // is Some) and just-finished (active now None → one last mark to render
    // the settled state, then the loop stops naturally).
    if was_active {
      ctx
        .window()
        .tree()
        .dirty_marker()
        .mark(ctx.widget_id(), self.required_dirty_phase());
    }
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let now = Instant::now();
    if !self.is_content_motion_active(animations_enabled(&ctx.window()), now) {
      return host.hit_test(ctx, pos);
    }

    let box_pos = ctx.box_pos().unwrap_or(Point::zero());
    let local_pos = pos - box_pos.to_vector();
    let animated_size = self.animated_size(now);
    let scale = self.scale_factor(now);

    match self.content_motion {
      ContentMotion::ClipReveal => {
        if !point_in_size(local_pos, animated_size) {
          HitTest { hit: false, can_hit_child: false }
        } else {
          host.hit_test(ctx, pos)
        }
      }
      ContentMotion::Scale => {
        let transform = Transform::scale(scale.x, scale.y);
        if let Some(inverse) = transform.inverse() {
          let mapped = inverse.transform_point(local_pos) + box_pos.to_vector();
          host.hit_test(ctx, mapped)
        } else {
          HitTest { hit: false, can_hit_child: false }
        }
      }
    }
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    let now = Instant::now();
    if self.content_motion != ContentMotion::Scale
      || !self.has_size_axes()
      || self.size_mode != SizeMode::Visual
      || !self.is_animating(now)
    {
      return host.get_transform();
    }

    let scale = self.scale_factor(now);
    if scale == Vector::one() {
      return host.get_transform();
    }

    let t = Transform::scale(scale.x, scale.y);
    if let Some(host_t) = host.get_transform() { Some(t.then(&host_t)) } else { Some(t) }
  }

  fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase {
    use DirtyPhase::*;
    match (self.required_dirty_phase(), host.dirty_phase()) {
      (LayoutSubtree, _) | (_, LayoutSubtree) => LayoutSubtree,
      (Layout, _) | (_, Layout) => Layout,
      (Position, _) | (_, Position) => Position,
      (Paint, Paint) => Paint,
    }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { self.required_dirty_phase() }
}

fn default_transition() -> Rc<Box<dyn Transition>> {
  Rc::new(Box::new(EasingTransition {
    easing: easing::LinearEasing,
    duration: Duration::from_millis(200),
  }))
}

impl SmoothLayoutDeclarer {
  /// Set the transition used to interpolate geometry.
  ///
  /// Defaults to a 200 ms linear ease when not specified.
  pub fn with_transition(&mut self, transition: impl Transition + 'static) -> &mut Self {
    self.transition = Some(Rc::new(Box::new(transition)));
    self
  }

  /// Set the initial size (both width and height) for the entry animation.
  ///
  /// Accepts [`Measure`] values, which can be absolute pixels or percentages
  /// of the containing box. Only used on first appearance.
  pub fn with_init_size(&mut self, init: impl Into<Size<Measure>>) -> &mut Self {
    let init = init.into();
    self.init_width = Some(Some(init.width));
    self.init_height = Some(Some(init.height));
    self
  }

  /// Set the initial X position for the entry animation.
  pub fn with_init_x(&mut self, init: impl Into<AnchorX>) -> &mut Self {
    let pos = self.init_pos.get_or_insert_with(Anchor::default);
    pos.x = Some(init.into());
    self
  }

  /// Set the initial Y position for the entry animation.
  pub fn with_init_y(&mut self, init: impl Into<AnchorY>) -> &mut Self {
    let pos = self.init_pos.get_or_insert_with(Anchor::default);
    pos.y = Some(init.into());
    self
  }

  /// Set the initial width for the entry animation.
  ///
  /// Accepts absolute pixels (`5.0_f32.into()`) or a percentage
  /// (`50.percent()`). Only used on first appearance.
  pub fn with_init_width(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_width = Some(Some(init.into()));
    self
  }

  /// Set the initial height for the entry animation.
  ///
  /// Accepts absolute pixels (`5.0_f32.into()`) or a percentage
  /// (`50.percent()`). Only used on first appearance.
  pub fn with_init_height(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_height = Some(Some(init.into()));
    self
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
      x: AnchorX::center(),
      y: AnchorY::center()
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
        clamp: BoxClamp::EXPAND_BOTH,
        @SmoothLayout {
          axes: SmoothAxes::POS,
          transition: TEST_TRANS,
          @center_red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::POS,
          transition: TEST_TRANS,
          init_pos: Anchor::left_top(5., 10.percent()),
          @center_red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::POS,
          transition: TEST_TRANS,
          init_pos: Anchor::right_bottom(10.percent(), 5.),
          @center_red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(100., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
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
        @SmoothLayout {
          axes: SmoothAxes::X,
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::X,
          transition: TEST_TRANS,
          init_x: 10.percent(),
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::X,
          transition: TEST_TRANS,
          init_x: AnchorX::right(),
          @red_block_10_x_10()
        }
        @Container {
          size: Size::new(100., 10.),
        }
      })
      .with_wnd_size(Size::new(100., 30.))
      .with_flags(WindowFlags::ANIMATIONS),
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
        @SmoothLayout {
          axes: SmoothAxes::Y,
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::Y,
          transition: TEST_TRANS,
          init_y: 10.percent(),
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::Y,
          transition: TEST_TRANS,
          init_y: AnchorY::bottom(),
          @red_block_10_x_10()
        }
        @Container { size: Size::new(10., 100.) }
      })
      .with_wnd_size(Size::new(30., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_y"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_size() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(self::smooth_layout! {
        axes: SmoothAxes::SIZE,
        transition: TEST_TRANS,
        init_size: Size::splat(50.percent()),
        @red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_size_from_50p"
    );

    assert_widget_eq_image!(
      WidgetTester::new(self::smooth_layout! {
        axes: SmoothAxes::SIZE,
        transition: TEST_TRANS,
        init_size: Size::splat(5f32.into()),
        @red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_size_from_5"
    );

    assert_widget_eq_image!(
      WidgetTester::new(self::smooth_layout! {
        axes: SmoothAxes::SIZE,
        transition: TEST_TRANS,
        @center_red_block_10_x_10()
      })
      .with_wnd_size(Size::new(100., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_size_from_real"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_width() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(flex! {
        direction: Direction::Vertical,
        item_gap: 2.,
        @SmoothLayout {
          axes: SmoothAxes::WIDTH,
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::WIDTH,
          transition: TEST_TRANS,
          init_width: 50.percent(),
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::WIDTH,
          transition: TEST_TRANS,
          init_width: 5.,
          @red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(100., 40.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_width"
    );
  }

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn smooth_height() {
    reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(flex! {
        item_gap: 2.,
        @SmoothLayout {
          axes: SmoothAxes::HEIGHT,
          transition: TEST_TRANS,
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::HEIGHT,
          transition: TEST_TRANS,
          init_height: 50.percent(),
          @red_block_10_x_10()
        }
        @SmoothLayout {
          axes: SmoothAxes::HEIGHT,
          transition: TEST_TRANS,
          init_height: 5.,
          @red_block_10_x_10()
        }
      })
      .with_wnd_size(Size::new(40., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_height"
    );
  }

  // Cross-platform smoke test: verifies that initialization and settling over
  // two frames does not cause panics for any axis type.
  #[test]
  fn smooth_init_no_panic() {
    reset_test_env!();

    for axes in [SmoothAxes::POS, SmoothAxes::SIZE, SmoothAxes::X, SmoothAxes::H] {
      let wnd = TestWindow::new(
        fn_widget! {
          @SmoothLayout {
            axes,
            transition: TEST_TRANS,
            init_pos: Anchor::left_top(5., 5.),
            init_size: Size::splat(5f32.into()),
            @center_red_block_10_x_10()
          }
        },
        Size::new(100., 100.),
        WindowFlags::ANIMATIONS,
      );
      wnd.draw_frame();
      wnd.draw_frame();
    }
  }

  // Child must be tappable at rest regardless of whether the ANIMATIONS flag
  // is set.  Both branches are exercised in a single test to make the
  // relationship explicit and avoid duplicating the widget definition.
  #[test]
  fn smooth_layout_keeps_child_tappable() {
    reset_test_env!();

    let tap = |flags: WindowFlags| {
      let tap_count = Stateful::new(0);
      let count_reader = tap_count.clone_reader();
      let wnd = TestWindow::new(
        fn_widget! {
          let tap_count = tap_count.clone_writer();
          @SmoothLayout {
            @MockBox {
              x: AnchorX::center(),
              y: AnchorY::center(),
              size: Size::new(100., 100.),
              on_tap: move |_| *$write(tap_count) += 1,
            }
          }
        },
        Size::new(500., 500.),
        flags,
      );
      wnd.draw_frame();
      wnd.process_cursor_move(Point::new(250., 250.));
      wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
      wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
      wnd.draw_frame();
      *count_reader.read()
    };

    assert_eq!(tap(WindowFlags::empty()), 1, "without ANIMATIONS flag");
    assert_eq!(tap(WindowFlags::ANIMATIONS), 1, "with ANIMATIONS flag");
  }

  fn smooth_layout_content_motion_fallback_after_finish(motion: ContentMotion) {
    const SHORT_TRANS: EasingTransition<LinearEasing> =
      EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(1) };

    let tap_count = Stateful::new(0);
    let count_reader = tap_count.clone_reader();

    let wnd = TestWindow::new(
      fn_widget! {
        let tap_count = tap_count.clone_writer();
        @SmoothLayout {
          axes: SmoothAxes::SIZE,
          content_motion: motion,
          transition: SHORT_TRANS,
          init_size: Size::splat(10f32.into()),
          @MockBox {
            size: Size::new(100., 100.),
            on_tap: move |_| *$write(tap_count) += 1,
          }
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );

    // First frame: animation starts from 10x10 -> 100x100.
    wnd.draw_frame();

    // Content motion should block this hit outside the animated (small) size.
    wnd.process_cursor_move(Point::new(50., 50.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    assert_eq!(*count_reader.read(), 0);

    // Wait for animation to finish.
    std::thread::sleep(Duration::from_millis(10));
    wnd.draw_frame();

    // After animation finishes, hit-test should fall back to host.
    wnd.process_cursor_move(Point::new(50., 50.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    assert_eq!(*count_reader.read(), 1);
  }

  #[test]
  fn smooth_layout_clip_reveal_only_active_while_motion_running() {
    reset_test_env!();
    smooth_layout_content_motion_fallback_after_finish(ContentMotion::ClipReveal);
  }

  #[test]
  fn smooth_layout_scale_only_active_while_motion_running() {
    reset_test_env!();
    smooth_layout_content_motion_fallback_after_finish(ContentMotion::Scale);
  }

  #[test]
  fn smooth_layout_visual_size_keeps_redrawing_while_animating() {
    reset_test_env!();

    const SHORT_TRANS: EasingTransition<LinearEasing> =
      EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(120) };

    let mut wnd = TestWindow::new(
      fn_widget! {
        @SmoothLayout {
          axes: SmoothAxes::SIZE,
          size_mode: SizeMode::Visual,
          content_motion: ContentMotion::ClipReveal,
          transition: SHORT_TRANS,
          init_size: Size::splat(10f32.into()),
          @MockBox {
            size: Size::new(100., 100.),
          }
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );

    // Initial frame starts the animation.
    wnd.draw_frame();
    let _ = wnd.take_last_frame();

    let mut redraw_count = 0;
    for _ in 0..6 {
      std::thread::sleep(Duration::from_millis(20));
      wnd.draw_frame();
      if wnd.take_last_frame().is_some() {
        redraw_count += 1;
      }
    }

    assert!(
      redraw_count >= 2,
      "expected smooth visual size animation to redraw across multiple frames, \
       redraw_count={redraw_count}",
    );
  }

  #[test]
  fn smooth_layout_layout_mode_updates_layout_size_while_animating() {
    reset_test_env!();

    const SHORT_TRANS: EasingTransition<LinearEasing> =
      EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(120) };

    let tracker = Stateful::new(None);
    let wnd = TestWindow::new(
      fn_widget! {
        @SmoothLayout {
          on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
          axes: SmoothAxes::SIZE,
          size_mode: SizeMode::Layout,
          transition: SHORT_TRANS,
          init_size: Size::splat(10f32.into()),
          @MockBox {
            size: Size::new(100., 100.),
          }
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );

    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    let size_1 = wnd.widget_size(id).unwrap();
    assert!(size_1.width < 100. && size_1.height < 100., "size_1={size_1:?}");

    std::thread::sleep(Duration::from_millis(20));
    wnd.draw_frame();
    let size_2 = wnd.widget_size(id).unwrap();
    assert!(size_2.width > size_1.width, "size_1={size_1:?}, size_2={size_2:?}");
    assert!(size_2.height > size_1.height, "size_1={size_1:?}, size_2={size_2:?}");
  }

  #[test]
  fn smooth_layout_visual_mode_keeps_target_layout_size_while_animating() {
    reset_test_env!();

    const SHORT_TRANS: EasingTransition<LinearEasing> =
      EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(120) };

    let tracker = Stateful::new(None);
    let wnd = TestWindow::new(
      fn_widget! {
        @SmoothLayout {
          on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
          axes: SmoothAxes::SIZE,
          size_mode: SizeMode::Visual,
          transition: SHORT_TRANS,
          init_size: Size::splat(10f32.into()),
          @MockBox {
            size: Size::new(100., 100.),
          }
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );

    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    let size_1 = wnd.widget_size(id).unwrap();
    assert_eq!(size_1, Size::new(100., 100.));

    std::thread::sleep(Duration::from_millis(20));
    wnd.draw_frame();
    let size_2 = wnd.widget_size(id).unwrap();
    assert_eq!(size_2, Size::new(100., 100.));
  }

  #[test]
  fn smooth_layout_x_should_not_jump_on_target_change() {
    reset_test_env!();

    let x = Stateful::new(0.);
    let x_writer = x.clone_writer();
    let tracker = Stateful::new(None);

    let wnd = TestWindow::new(
      fn_widget! {
        let x_writer = x_writer.clone_writer();
        @MockBox {
          size: Size::new(300., 100.),
          @SmoothLayout {
            on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
            axes: SmoothAxes::X,
            transition: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(200),
            },
            @MockBox {
              size: Size::new(10., 10.),
              x: pipe!(AnchorX::left().offset(*$read(x_writer))),
            }
          }
        }
      },
      Size::new(300., 100.),
      WindowFlags::ANIMATIONS,
    );

    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    assert_eq!(wnd.widget_pos(id).unwrap().x, 0.);

    // Let init phase settle.
    wnd.draw_frame();

    *x.write() = 200.;
    wnd.draw_frame();

    let x_after_1 = wnd.widget_pos(id).unwrap().x;
    assert!((0. ..200.).contains(&x_after_1), "x_after_1={x_after_1}");

    for _ in 0..40 {
      wnd.draw_frame();
    }
    let x_later = wnd.widget_pos(id).unwrap().x;
    assert!(x_later > x_after_1, "x_after_1={x_after_1}, x_later={x_later}");
  }
}
