//! Smooth layout widgets that animate position and/or size transitions between
//! layout updates.
//!
//! # Overview
//!
//! This module provides two public widgets with distinct responsibilities:
//!
//! - [`SmoothLayout`] — the primary widget for local position and size
//!   smoothing in the current parent coordinate space.
//! - [`SmoothGlobal`] — a specialized widget for global position smoothing in
//!   window coordinates.
//!
//! Both widgets intercept the layout pipeline and interpolate geometry from a
//! *from* value toward a *target* value on each frame. The animation runs
//! entirely inside the layout/paint pipeline, and dirty marking is
//! self-scheduled via `paint()`.
//!
//! # `SmoothLayout`
//!
//! Position and size are configured independently:
//! - [`PosAxes`] controls which position axes participate (`X`, `Y`, or `Pos`
//!   for both). Position is always in the parent-local space.
//! - [`SizeAxes`] controls which size axes (`W`/`H`) participate.
//!
//! The default smooths both position axes locally and both size axes:
//! - `pos_axes = PosAxes::Pos`
//! - `size_axes = SizeAxes::Size`
//!
//! ## Position smoothing (`PosAxes`)
//!
//! For position animation, **bind the dynamic `x`/`y` on the child widget**,
//! not on `SmoothLayout` itself. `SmoothLayout` reads the position that the
//! parent assigns (via `adjust_position`) and interpolates it frame-by-frame.
//!
//! ```rust,ignore
//! @SmoothLayout {
//!   pos_axes: PosAxes::Pos,
//!   size_axes: SizeAxes::None,
//!   // Optional: where to start from on first appearance
//!   init_pos: Anchor::left_top(0., 100.),
//!   @MyWidget {
//!     // Moving this x drives the smooth animation
//!     x: pipe!(AnchorX::left().offset(*$read(offset))),
//!   }
//! }
//! ```
//!
//! ## Size smoothing (`SizeAxes::Size`, `SizeAxes::Width`, `SizeAxes::Height`)
//!
//! For size animation, the behaviour visible to the rest of the layout is
//! governed by [`SizeMode`]:
//!
//! | [`SizeMode`]               | Effect                                             |
//! |----------------------------|----------------------------------------------------|
//! | [`SizeMode::Visual`]       | Layout reports *target* size; animation is visual only. |
//! | [`SizeMode::Layout`]       | Layout reports *animated* size and relayouts the child. |
//!
//! The visual effect during animation is set by [`SizeEffect`]:
//!
//! | [`SizeEffect`]             | Effect                                             |
//! |-----------------------------|----------------------------------------------------|
//! | [`SizeEffect::Clip`]        | Clips paint output to animated size (default).   |
//! | [`SizeEffect::Scale`]       | Scales content from basis size to animated size. |
//!
//! ```rust,ignore
//! // Reveal a widget by expanding from 0 width
//! @SmoothLayout {
//!   pos_axes: PosAxes::None,
//!   size_axes: SizeAxes::Width,
//!   init_width: 0.,
//!   @MyWidget {}
//! }
//!
//! // Smooth size transition with scale effect
//! @SmoothLayout {
//!   pos_axes: PosAxes::None,
//!   size_axes: SizeAxes::Size,
//!   size_effect: SizeEffect::Scale,
//!   init_size: Size::splat(0f32.into()),
//!   @MyWidget {}
//! }
//! ```
//!
//! # `SmoothGlobal`
//!
//! Global position smoothing is a specialized capability for cross-subtree or
//! shared overlay motion where continuity should follow the widget's visible
//! position in the window, rather than its coordinates inside the current
//! parent.
//!
//! ```rust,ignore
//! @SmoothGlobal {
//!   pos_axes: PosAxes::Y,
//!   transition: ...,
//!   @Child {}
//! }
//! ```
//!
//! If you need both global-position motion and size motion, nest the widgets:
//!
//! ```rust,ignore
//! @SmoothGlobal {
//!   pos_axes: PosAxes::Y,
//!   @SmoothLayout {
//!     pos_axes: PosAxes::None,
//!     size_axes: SizeAxes::Size,
//!     @Child {}
//!   }
//! }
//! ```
//!
//! # Initial value
//!
//! On *first appearance* you can specify where to animate from via:
//! - `init_pos` / `init_x` / `init_y` — initial position ([`Anchor`]).
//! - `init_size` / `init_width` / `init_height` — initial size ([`Measure`],
//!   accepts pixels or percentages of the containing box). (SmoothLayout only)
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

use std::cell::{Cell, RefCell};

use rxrust::subscription::BoxedSubscription;

use crate::{prelude::*, ticker::FrameMsg, window::WindowFlags, wrap_render::*};

/// Controls how animated size changes interact with layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeMode {
  /// Layout reports *target* size; animation is visual only.
  Visual,
  /// Layout reports *animated* size and relayouts the child.
  #[default]
  Layout,
}

/// Visual effect applied during size animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeEffect {
  /// Clips paint output to animated size.
  #[default]
  Clip,
  /// Scales content from basis size to animated size.
  Scale,
}

/// Selects which position axes participate in smooth interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PosAxes {
  #[default]
  None,
  /// Horizontal position (X offset).
  X,
  /// Vertical position (Y offset).
  Y,
  /// Both position axes (`X` and `Y`).
  Pos,
}

impl PosAxes {
  fn has_x(self) -> bool { matches!(self, Self::X | Self::Pos) }
  fn has_y(self) -> bool { matches!(self, Self::Y | Self::Pos) }
}

/// Selects which size axes participate in smooth interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeAxes {
  #[default]
  None,
  /// Width.
  Width,
  /// Height.
  Height,
  /// Both size axes (`Width` and `Height`).
  Size,
}

impl SizeAxes {
  fn has_width(self) -> bool { matches!(self, Self::Width | Self::Size) }
  fn has_height(self) -> bool { matches!(self, Self::Height | Self::Size) }
}

/// A wrapper widget that smoothly animates position and/or size transitions
/// between layout updates.
#[declare(stateless)]
pub struct SmoothLayout {
  #[declare(default = PosAxes::Pos)]
  pos_axes: PosAxes,
  #[declare(default = SizeAxes::Size)]
  size_axes: SizeAxes,
  #[declare(default)]
  size_mode: SizeMode,
  #[declare(default)]
  size_effect: SizeEffect,
  #[declare(custom, default = default_transition())]
  transition: Rc<Box<dyn Transition>>,
  #[declare(default)]
  init_pos: Anchor,
  #[declare(custom, default)]
  init_width: Option<Measure>,
  #[declare(custom, default)]
  init_height: Option<Measure>,
  #[declare(skip, default = Rc::new(Cell::new(MotionState::default())))]
  motion: Rc<Cell<MotionState<Rect>>>,
}

#[derive(Clone, Copy)]
struct ActiveTween<T> {
  from: T,
  to: T,
  started_at: Instant,
}

#[derive(Clone, Copy, Default)]
struct MotionState<T> {
  target: T,
  initialized: bool,
  active: Option<ActiveTween<T>>,
}

#[derive(Clone, Copy, Default)]
struct GlobalRuntime {
  motion: MotionState<Point>,
  target_size: Size,
  presented_global: Option<Point>,
}

fn update_cell<T: Copy, R>(cell: &Cell<T>, f: impl FnOnce(&mut T) -> R) -> R {
  let mut value = cell.get();
  let result = f(&mut value);
  cell.set(value);
  result
}

fn boxed_transition(t: impl Transition + 'static) -> Rc<Box<dyn Transition>> {
  Rc::new(Box::new(t))
}

fn init_anchor(init_pos: &mut Option<Anchor>) -> &mut Anchor {
  init_pos.get_or_insert_with(Anchor::default)
}

fn sample_motion<T: Copy>(
  motion: &mut MotionState<T>, transition: &dyn Transition, now: Instant,
  lerp: impl Fn(&T, &T, f32) -> T,
) -> T {
  let Some(active) = motion.active else { return motion.target };

  let progress = transition.rate_of_change(now - active.started_at);
  if progress.is_finish() {
    motion.active = None;
    active.to
  } else {
    lerp(&active.from, &active.to, progress.value())
  }
}

fn start_motion<T: Copy + PartialEq>(
  motion: &mut MotionState<T>, from: T, to: T, now: Instant, enabled: bool,
) {
  if !enabled || from == to {
    motion.active = None;
  } else {
    motion.active = Some(ActiveTween { from, to, started_at: now });
  }
}

fn restart_motion<T: Copy + PartialEq>(
  motion: &mut MotionState<T>, current: T, target: T, now: Instant, enabled: bool,
) {
  motion.target = target;
  start_motion(motion, current, target, now, enabled);
}

fn initialize_motion<T: Copy + PartialEq>(
  motion: &mut MotionState<T>, from: Option<T>, now: Instant, enabled: bool,
) {
  let target = motion.target;
  motion.initialized = true;
  start_motion(motion, from.unwrap_or(target), target, now, enabled);
}

fn resolve_init_origin(
  target: Point, size: Size, clamp: BoxClamp, axes: PosAxes, init_pos: &Anchor,
) -> Point {
  let mut origin = target;
  let max = Size::new(clamp.container_width(size.width), clamp.container_height(size.height));
  if let Some(anchor) = init_pos.x.as_ref().filter(|_| axes.has_x()) {
    origin.x = anchor.calculate(max.width, size.width);
  }
  if let Some(anchor) = init_pos.y.as_ref().filter(|_| axes.has_y()) {
    origin.y = anchor.calculate(max.height, size.height);
  }
  origin
}

fn resolve_init_size(
  target: Size, clamp: BoxClamp, axes: SizeAxes, init_width: Option<Measure>,
  init_height: Option<Measure>,
) -> Size {
  let mut size = target;
  if let Some(width) = init_width.filter(|_| axes.has_width()) {
    size.width = width.into_pixel(clamp.max.width);
  }
  if let Some(height) = init_height.filter(|_| axes.has_height()) {
    size.height = height.into_pixel(clamp.max.height);
  }
  size
}

fn lerp_point_axes(from: &Point, to: &Point, factor: f32, axes: PosAxes) -> Point {
  let mut out = *to;
  if axes.has_x() {
    out.x = from.x.lerp(&to.x, factor);
  }
  if axes.has_y() {
    out.y = from.y.lerp(&to.y, factor);
  }
  out
}

fn lerp_size_axes(from: &Size, to: &Size, factor: f32, axes: SizeAxes) -> Size {
  let mut out = *to;
  if axes.has_width() {
    out.width = from.width.lerp(&to.width, factor);
  }
  if axes.has_height() {
    out.height = from.height.lerp(&to.height, factor);
  }
  out
}

fn lerp_rect_axes(from: &Rect, to: &Rect, factor: f32, pos: PosAxes, size: SizeAxes) -> Rect {
  Rect::new(
    lerp_point_axes(&from.origin, &to.origin, factor, pos),
    lerp_size_axes(&from.size, &to.size, factor, size),
  )
}

fn constrain_to_animated_axes(clamp: BoxClamp, size: Size, axes: SizeAxes) -> BoxClamp {
  let mut constrained = clamp;
  if axes.has_width() {
    constrained.min.width = size.width;
    constrained.max.width = size.width;
  }
  if axes.has_height() {
    constrained.min.height = size.height;
    constrained.max.height = size.height;
  }
  constrained
}

fn animations_on(w: &Window) -> bool { w.flags().contains(WindowFlags::ANIMATIONS) }

fn to_global(w: &Window, p: Option<WidgetId>, pos: Point) -> Point {
  p.map_or(pos, |p| w.map_to_global(pos, p))
}

fn to_local(w: &Window, p: Option<WidgetId>, pos: Point) -> Point {
  p.map_or(pos, |p| w.map_from_global(pos, p))
}

impl SmoothLayout {
  fn has_pos_axes(&self) -> bool { self.pos_axes != PosAxes::None }

  fn has_size_axes(&self) -> bool { self.size_axes != SizeAxes::None }

  fn has_motion_axes(&self) -> bool { self.has_pos_axes() || self.has_size_axes() }

  fn transition(&self) -> &dyn Transition { self.transition.as_ref().as_ref() }

  fn motion_enabled(&self, window: &Window) -> bool {
    self.has_motion_axes() && animations_on(window)
  }

  fn with_motion<R>(&self, f: impl FnOnce(&mut MotionState<Rect>) -> R) -> R {
    update_cell(self.motion.as_ref(), f)
  }

  fn sample_rect(&self, motion: &mut MotionState<Rect>, now: Instant) -> Rect {
    sample_motion(motion, self.transition(), now, |from, to, factor| {
      lerp_rect_axes(from, to, factor, self.pos_axes, self.size_axes)
    })
  }

  fn presented_rect(&self, now: Instant) -> Rect {
    self.with_motion(|motion| self.sample_rect(motion, now))
  }

  fn is_animating(&self, now: Instant) -> bool {
    self.with_motion(|motion| {
      self.sample_rect(motion, now);
      motion.active.is_some()
    })
  }

  fn target_size(&self) -> Size { self.motion.get().target.size }

  fn animated_size(&self, now: Instant) -> Size { self.presented_rect(now).size }

  fn scale_factor(&self, now: Instant) -> Vector {
    let target = self.target_size();
    let animated = self.animated_size(now);
    let sx = if target.width > 0. { animated.width / target.width } else { 1. };
    let sy = if target.height > 0. { animated.height / target.height } else { 1. };
    Vector::new(sx, sy)
  }

  fn required_dirty_phase(&self) -> DirtyPhase {
    if !self.has_size_axes() && self.has_pos_axes() {
      return DirtyPhase::Position;
    }
    if self.has_size_axes() && self.size_mode == SizeMode::Visual && !self.has_pos_axes() {
      return DirtyPhase::Paint;
    }
    DirtyPhase::Layout
  }

  fn is_size_effect_active(&self, animations_on: bool, now: Instant) -> bool {
    self.has_size_axes()
      && self.size_mode == SizeMode::Visual
      && animations_on
      && self.is_animating(now)
  }

  fn resolve_init_rect(&self, target: Rect, clamp: BoxClamp) -> Option<Rect> {
    let size =
      resolve_init_size(target.size, clamp, self.size_axes, self.init_width, self.init_height);
    let origin = resolve_init_origin(target.origin, size, clamp, self.pos_axes, &self.init_pos);
    let from = Rect::new(origin, size);
    (from != target).then_some(from)
  }
}

impl<'c> ComposeChild<'c> for SmoothLayout {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for SmoothLayout {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    let target = host.measure(clamp, ctx);
    let now = Instant::now();
    let enabled = self.motion_enabled(&ctx.window());

    self.with_motion(|motion| {
      if motion.initialized {
        if target != motion.target.size {
          let current = self.sample_rect(motion, now);
          restart_motion(motion, current, Rect::new(motion.target.origin, target), now, enabled);
        }
      } else {
        motion.target.size = target;
        if !self.has_pos_axes() {
          let from = self.resolve_init_rect(motion.target, clamp);
          initialize_motion(motion, from, now, enabled);
        }
      }
    });

    let layout_size = if self.has_size_axes() && self.size_mode == SizeMode::Layout {
      self.animated_size(now)
    } else {
      target
    };

    if self.has_size_axes() && self.size_mode == SizeMode::Layout && layout_size != target {
      host.measure(constrain_to_animated_axes(clamp, layout_size, self.size_axes), ctx)
    } else {
      layout_size
    }
  }

  fn place_children(&self, size: Size, host: &dyn Render, ctx: &mut PlaceCtx) {
    let place_size = if self.has_size_axes() && self.size_mode == SizeMode::Visual {
      self.target_size()
    } else {
      size
    };
    host.place_children(place_size, ctx)
  }

  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    let target_pos = host.adjust_position(pos, ctx);
    let now = Instant::now();
    let enabled = self.motion_enabled(&ctx.window());

    self.with_motion(|motion| {
      if motion.initialized {
        if target_pos != motion.target.origin {
          let current = self.sample_rect(motion, now);
          restart_motion(motion, current, Rect::new(target_pos, motion.target.size), now, enabled);
        }
      } else {
        motion.target.origin = target_pos;
        let from = self.resolve_init_rect(motion.target, ctx.clamp());
        initialize_motion(motion, from, now, enabled);
      }
    });

    self.presented_rect(now).origin
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let now = Instant::now();
    let was_active = self.motion.get().active.is_some();

    if self.is_size_effect_active(animations_on(&ctx.window()), now) {
      match self.size_effect {
        SizeEffect::Clip => {
          let rect = Rect::from_size(self.animated_size(now));
          ctx.box_painter().clip(Path::rect(&rect).into());
        }
        SizeEffect::Scale => {
          let scale = self.scale_factor(now);
          if scale != Vector::one() {
            ctx.painter().scale(scale.x, scale.y);
          }
        }
      }
    }

    host.paint(ctx);
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
    if !self.is_size_effect_active(animations_on(&ctx.window()), now) {
      return host.hit_test(ctx, pos);
    }

    let box_pos = ctx.box_pos().unwrap_or(Point::zero());
    let local_pos = pos - box_pos.to_vector();
    let animated = self.animated_size(now);

    match self.size_effect {
      SizeEffect::Clip => {
        if local_pos.x < 0.
          || local_pos.y < 0.
          || local_pos.x > animated.width
          || local_pos.y > animated.height
        {
          HitTest { hit: false, can_hit_child: false }
        } else {
          host.hit_test(ctx, pos)
        }
      }
      SizeEffect::Scale => {
        let scale = self.scale_factor(now);
        Transform::scale(scale.x, scale.y)
          .inverse()
          .map_or(HitTest { hit: false, can_hit_child: false }, |inv| {
            host.hit_test(ctx, inv.transform_point(local_pos) + box_pos.to_vector())
          })
      }
    }
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    if self.size_effect != SizeEffect::Scale
      || self.size_mode != SizeMode::Visual
      || !self.has_size_axes()
    {
      return host.get_transform();
    }

    let now = Instant::now();
    if !self.is_animating(now) {
      return host.get_transform();
    }

    let scale = self.scale_factor(now);
    if scale == Vector::one() {
      return host.get_transform();
    }

    let transform = Transform::scale(scale.x, scale.y);
    host
      .get_transform()
      .map_or(Some(transform), |host_transform| Some(transform.then(&host_transform)))
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
  boxed_transition(EasingTransition {
    easing: easing::LinearEasing,
    duration: Duration::from_millis(200),
  })
}

impl SmoothLayoutDeclarer {
  /// Set the transition used to interpolate geometry.
  pub fn with_transition(&mut self, t: impl Transition + 'static) -> &mut Self {
    self.transition = Some(boxed_transition(t));
    self
  }

  /// Set the initial size (both width and height) for the entry animation.
  pub fn with_init_size(&mut self, init: impl Into<Size<Measure>>) -> &mut Self {
    let init = init.into();
    self.init_width = Some(Some(init.width));
    self.init_height = Some(Some(init.height));
    self
  }

  /// Set the initial X position for the entry animation.
  pub fn with_init_x(&mut self, init: impl Into<AnchorX>) -> &mut Self {
    init_anchor(&mut self.init_pos).x = Some(init.into());
    self
  }

  /// Set the initial Y position for the entry animation.
  pub fn with_init_y(&mut self, init: impl Into<AnchorY>) -> &mut Self {
    init_anchor(&mut self.init_pos).y = Some(init.into());
    self
  }

  /// Set the initial width for the entry animation.
  pub fn with_init_width(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_width = Some(Some(init.into()));
    self
  }

  /// Set the initial height for the entry animation.
  pub fn with_init_height(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_height = Some(Some(init.into()));
    self
  }
}

// ---------------------------------------------------------------------------
// SmoothGlobal
// ---------------------------------------------------------------------------

/// A wrapper widget that smoothly animates position transitions in
/// window-global coordinates.
#[declare(stateless)]
pub struct SmoothGlobal {
  #[declare(default = PosAxes::Pos)]
  pos_axes: PosAxes,
  #[declare(custom, default = default_transition())]
  transition: Rc<Box<dyn Transition>>,
  #[declare(default)]
  init_pos: Anchor,
  #[declare(skip, default = Rc::new(Cell::new(GlobalRuntime::default())))]
  runtime: Rc<Cell<GlobalRuntime>>,
  #[declare(skip, default = Rc::new(RefCell::new(None)))]
  layout_ready_sub: Rc<RefCell<Option<BoxedSubscription>>>,
}

impl SmoothGlobal {
  fn transition(&self) -> &dyn Transition { self.transition.as_ref().as_ref() }

  fn has_pos_axes(&self) -> bool { self.pos_axes != PosAxes::None }

  fn motion_enabled(&self, window: &Window) -> bool { self.has_pos_axes() && animations_on(window) }

  fn with_runtime<R>(&self, f: impl FnOnce(&mut GlobalRuntime) -> R) -> R {
    update_cell(self.runtime.as_ref(), f)
  }

  fn sample_point(&self, motion: &mut MotionState<Point>, now: Instant) -> Point {
    sample_motion(motion, self.transition(), now, |from, to, factor| {
      lerp_point_axes(from, to, factor, self.pos_axes)
    })
  }

  fn refresh_presented_point(&self, now: Instant) -> Point {
    self.with_runtime(|runtime| {
      let presented = self.sample_point(&mut runtime.motion, now);
      runtime.presented_global = Some(presented);
      presented
    })
  }

  fn resolve_init_point(&self, target: Point, size: Size, clamp: BoxClamp) -> Option<Point> {
    let from = resolve_init_origin(target, size, clamp, self.pos_axes, &self.init_pos);
    (from != target).then_some(from)
  }

  fn ensure_tracking(&self, window: &Rc<Window>, widget_id: WidgetId) {
    if self.layout_ready_sub.borrow().is_some() {
      return;
    }

    let runtime = self.runtime.clone();
    let tracked_window = window.clone();
    let sub = window
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)))
      .subscribe(move |_| {
        let Some(local) = tracked_window.widget_pos(widget_id) else { return };
        let parent = tracked_window.parent(widget_id);
        update_cell(runtime.as_ref(), |state| {
          state.presented_global = Some(to_global(&tracked_window, parent, local));
        });
      });
    *self.layout_ready_sub.borrow_mut() = Some(BoxedSubscription::new(sub));
  }
}

impl Drop for SmoothGlobal {
  fn drop(&mut self) {
    if let Some(sub) = self.layout_ready_sub.borrow_mut().take() {
      sub.unsubscribe();
    }
  }
}

impl<'c> ComposeChild<'c> for SmoothGlobal {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for SmoothGlobal {
  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    let target_pos = host.adjust_position(pos, ctx);
    let window = ctx.window();
    let parent = ctx.parent();
    let target = to_global(&window, parent, target_pos);
    let now = Instant::now();
    let enabled = self.motion_enabled(&window);

    self.ensure_tracking(&window, ctx.widget_id());

    self.with_runtime(|runtime| {
      if runtime.motion.initialized {
        if target != runtime.motion.target {
          let current = runtime
            .presented_global
            .unwrap_or_else(|| self.sample_point(&mut runtime.motion, now));
          restart_motion(&mut runtime.motion, current, target, now, enabled);
        }
      } else {
        runtime.motion.target = target;
        let local_target = to_local(&window, parent, target);
        let from = self
          .resolve_init_point(local_target, runtime.target_size, ctx.clamp())
          .map(|local_from| to_global(&window, parent, local_from));
        initialize_motion(&mut runtime.motion, from, now, enabled);
      }
    });

    to_local(&window, parent, self.refresh_presented_point(now))
  }

  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    let size = host.measure(clamp, ctx);
    self.with_runtime(|runtime| runtime.target_size = size);
    size
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let was_active = self.runtime.get().motion.active.is_some();
    host.paint(ctx);
    if was_active {
      self.refresh_presented_point(Instant::now());
      ctx
        .window()
        .tree()
        .dirty_marker()
        .mark(ctx.widget_id(), DirtyPhase::Position);
    }
  }

  fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase {
    use DirtyPhase::*;
    match host.dirty_phase() {
      LayoutSubtree => LayoutSubtree,
      Layout => Layout,
      Paint | Position => Position,
    }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Position }
}

impl SmoothGlobalDeclarer {
  /// Set the transition used to interpolate position.
  pub fn with_transition(&mut self, t: impl Transition + 'static) -> &mut Self {
    self.transition = Some(boxed_transition(t));
    self
  }

  /// Set the initial X position for the entry animation.
  pub fn with_init_x(&mut self, init: impl Into<AnchorX>) -> &mut Self {
    init_anchor(&mut self.init_pos).x = Some(init.into());
    self
  }

  /// Set the initial Y position for the entry animation.
  pub fn with_init_y(&mut self, init: impl Into<AnchorY>) -> &mut Self {
    init_anchor(&mut self.init_pos).y = Some(init.into());
    self
  }
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;

  use ribir::{
    core::{reset_test_env, test_helper::*, window::WindowFlags},
    prelude::{easing::LinearEasing, *},
  };
  use ribir_dev_helper::*;

  const TEST_TRANS: EasingTransition<LinearEasing> =
    EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(200) };

  #[derive(Clone)]
  struct StepTransition {
    progress: Rc<Cell<AnimateProgress>>,
  }

  impl Transition for StepTransition {
    fn rate_of_change(&self, _: Duration) -> AnimateProgress { self.progress.get() }
    fn duration(&self) -> Duration { Duration::from_millis(1) }
    fn dyn_clone(&self) -> Box<dyn Transition> { Box::new(self.clone()) }
  }

  fn red_block() -> Widget<'static> {
    container! { background: Color::RED, size: Size::new(10., 10.) }.into_widget()
  }

  fn centered_red_block() -> Widget<'static> {
    container! {
      background: Color::RED,
      size: Size::new(10., 10.),
      x: AnchorX::center(),
      y: AnchorY::center(),
    }
    .into_widget()
  }

  fn reused_smooth_global(
    tracker: impl StateWriter<Value = Option<WidgetId>>, progress: Rc<Cell<AnimateProgress>>,
  ) -> Widget<'static> {
    fn_widget! {
      let progress = progress.clone();
      @Reuse {
        reuse_id: GlobalId::new("smooth_global_reuse"),
        @SmoothGlobal {
          on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
          pos_axes: PosAxes::X,
          transition: StepTransition { progress },
          @red_block()
        }
      }
    }
    .into_widget()
  }

  #[test]
  fn smooth_pos() {
    reset_test_env!();
    assert_widget_eq_image!(
      WidgetTester::new(stack! {
        clamp: BoxClamp::EXPAND_BOTH,
        @SmoothLayout {
          pos_axes: PosAxes::Pos,
          size_axes: SizeAxes::None,
          transition: TEST_TRANS,
          @centered_red_block()
        }
        @SmoothLayout {
          pos_axes: PosAxes::Pos,
          size_axes: SizeAxes::None,
          transition: TEST_TRANS,
          init_pos: Anchor::left_top(5., 10.percent()),
          @centered_red_block()
        }
        @SmoothLayout {
          pos_axes: PosAxes::Pos,
          size_axes: SizeAxes::None,
          transition: TEST_TRANS,
          init_pos: Anchor::right_bottom(10.percent(), 5.),
          @centered_red_block()
        }
      })
      .with_wnd_size(Size::new(100., 100.))
      .with_flags(WindowFlags::ANIMATIONS),
      "smooth_pos"
    );
  }

  #[test]
  fn smooth_init_no_panic() {
    reset_test_env!();
    for (pos_axes, size_axes) in [(PosAxes::Pos, SizeAxes::None), (PosAxes::None, SizeAxes::Size)] {
      let wnd = TestWindow::new(
        fn_widget! {
          @SmoothLayout {
            pos_axes,
            size_axes,
            transition: TEST_TRANS,
            init_pos: Anchor::left_top(5., 5.),
            init_size: Size::splat(5f32.into()),
            @centered_red_block()
          }
        },
        Size::new(100., 100.),
        WindowFlags::ANIMATIONS,
      );
      wnd.draw_frame();
      wnd.draw_frame();
    }
  }

  #[test]
  fn smooth_layout_defaults_smoothing() {
    reset_test_env!();
    let progress = Rc::new(Cell::new(AnimateProgress::Finish));
    let tracker = Stateful::new(None);
    let offset = Stateful::new(0.);
    let p_inner = progress.clone();
    let wnd = TestWindow::new(
      fn_widget! {
        let p_inner = p_inner.clone();
        @MockBox {
          size: Size::new(200., 200.),
          @SmoothLayout {
            on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
            transition: StepTransition { progress: p_inner },
            @MockBox {
              size: Size::new(100., 100.),
              x: pipe!(AnchorX::left().offset(*$read(offset))),
              y: pipe!(AnchorY::top().offset(*$read(offset))),
            }
          }
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );
    wnd.draw_frame();
    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    assert_eq!(wnd.widget_pos(id).unwrap(), Point::zero());
    *offset.write() = 100.;
    progress.set(AnimateProgress::Between(0.5));
    wnd.draw_frame();
    let pos = wnd.widget_pos(id).unwrap();
    assert!((0. ..100.).contains(&pos.x));
  }

  #[test]
  fn smooth_layout_mode_updates_size() {
    reset_test_env!();
    let tracker = Stateful::new(None);
    let progress = Rc::new(Cell::new(AnimateProgress::Between(0.1)));
    let p_inner = progress.clone();
    let wnd = TestWindow::new(
      fn_widget! {
        let p_inner = p_inner.clone();
        @SmoothLayout {
          on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
          pos_axes: PosAxes::None,
          size_axes: SizeAxes::Size,
          size_mode: SizeMode::Layout,
          transition: StepTransition { progress: p_inner },
          init_size: Size::splat(5f32.into()),
          @red_block()
        }
      },
      Size::new(200., 200.),
      WindowFlags::ANIMATIONS,
    );
    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    let s1 = wnd.widget_size(id).unwrap();
    assert!(s1.width < 10. && s1.width > 5.);
    progress.set(AnimateProgress::Between(0.8));
    wnd.draw_frame();
    let s2 = wnd.widget_size(id).unwrap();
    assert!(s2.width > s1.width);
  }

  #[test]
  fn smooth_global_reuse_continuity() {
    reset_test_env!();
    let tracker = Stateful::new(None);
    let progress = Rc::new(Cell::new(AnimateProgress::Finish));
    let place_on_right = Stateful::new(false);
    let tracker_for_widget = tracker.clone_writer();
    let progress_for_widget = progress.clone();
    let wnd = TestWindow::new(
      fn_widget! {
        let tracker = tracker_for_widget.clone_writer();
        let progress = progress_for_widget.clone();
        @MockBox {
          size: Size::new(200., 100.),
          @ {
            if *$read(place_on_right) {
              @MockBox {
                size: Size::new(10., 10.),
                x: 120.,
                @reused_smooth_global(tracker.clone_writer(), progress.clone())
              }
            } else {
              @MockBox {
                size: Size::new(10., 10.),
                @reused_smooth_global(tracker.clone_writer(), progress.clone())
              }
            }
          }
        }
      },
      Size::new(200., 100.),
      WindowFlags::ANIMATIONS,
    );
    wnd.draw_frame();
    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    assert_eq!(wnd.map_to_global(Point::zero(), id).x, 0.);
    *place_on_right.write() = true;
    progress.set(AnimateProgress::Between(0.5));
    wnd.draw_frame();
    let x = wnd.map_to_global(Point::zero(), id).x;
    assert!((0. ..120.).contains(&x));
  }

  #[test]
  fn smooth_global_scroll_motion() {
    reset_test_env!();
    let tracker = Stateful::new(None);
    let scroll = Stateful::new(None::<Box<dyn StateWriter<Value = ScrollableWidget>>>);
    let wnd = TestWindow::new(
      fn_widget! {
        @ScrollableWidget {
          scrollable: Scrollable::Y,
          on_mounted: move |e| *$write(scroll) = ScrollableWidget::writer_of(e),
          @MockBox {
            size: Size::new(100., 300.),
            @MockBox {
              size: Size::new(10., 10.),
              y: 120.,
              @SmoothGlobal {
                on_mounted: move |e| *$write(tracker) = Some(e.current_target()),
                pos_axes: PosAxes::Y,
                transition: TEST_TRANS,
                @red_block()
              }
            }
          }
        }
      },
      Size::new(100., 100.),
      WindowFlags::ANIMATIONS,
    );
    wnd.draw_frame();
    wnd.draw_frame();
    let id = (*tracker.read()).unwrap();
    assert_eq!(wnd.map_to_global(Point::zero(), id).y, 120.);
    scroll
      .read()
      .as_ref()
      .unwrap()
      .write()
      .jump_to(Point::new(0., 60.));
    wnd.draw_frame();
    let y = wnd.map_to_global(Point::zero(), id).y;
    assert!((60. ..120.).contains(&y));
  }
}
