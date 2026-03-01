//! Smooth layout widgets that animate position and/or size transitions between
//! layout updates.
//!
//! # Overview
//!
//! [`SmoothLayout`] is a single wrapper widget that intercepts the layout
//! pipeline and interpolates geometry — position and/or size — from a *from*
//! value toward a *target* value on each frame. The animation runs entirely
//! inside the layout/paint pipeline: no reactive subscription chain is needed,
//! and dirty marking is self-scheduled through `once_before_layout`.
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
//! governed by [`LayoutImpact`]:
//!
//! | [`LayoutImpact`]            | Effect                                             |
//! |-----------------------------|----------------------------------------------------|
//! | [`LayoutImpact::NoLayout`]    | Layout reports *target* size; visual only.        |
//! | [`LayoutImpact::SelfLayout`]  | Layout reports *animated* size (default).         |
//! | [`LayoutImpact::SubtreeLayout`]| Child also lays out at current animated size.    |
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

use crate::{prelude::*, widget_tree::WidgetId, window::WindowFlags, wrap_render::*};

/// Controls how the animated size affects the layout of surrounding widgets.
///
/// This setting only applies when size axes (`W`/`H`) are active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutImpact {
  /// Keep the reported layout size at the *target* (final) size at all times.
  /// The animation is purely visual: neighbours are never reflowed, and the
  /// child always lays out at target size. This is the lightest option.
  NoLayout,
  /// Report the *animated* size to the parent, causing surrounding siblings to
  /// shift as the size changes, but keep the child's own layout using the
  /// *target* size (so the child is not recalculated on every frame).
  ///
  /// This is the default.
  #[default]
  SelfLayout,
  /// Report the *animated* size **and** constrain the child to lay out within
  /// the animated size on every frame. Produces the most faithful animation
  /// for content that reflows (e.g. text), at the cost of a full subtree
  /// relayout per frame.
  SubtreeLayout,
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
/// `once_before_layout` so the widget keeps requesting redraws for as long as
/// the animation is running.
///
/// See the [module-level documentation](self) for a full usage guide.
pub struct SmoothLayout {
  axes: SmoothAxes,
  layout_impact: LayoutImpact,
  content_motion: ContentMotion,
  transition: Box<dyn Transition>,
  init_pos: Anchor,
  init_width: Option<Measure>,
  init_height: Option<Measure>,
  /// Interpolation endpoints, owned directly (never accessed from closures).
  target: Cell<Rect>,
  from: Cell<Rect>,
  /// True once the first layout has completed.
  layout_settled: Cell<bool>,
  /// Shared with `once_before_layout` closures: phase and schedule-guard.
  anim: Rc<SharedAnimState>,
}

/// Creates a `fn_widget` with [`SmoothLayout`] as its root widget.
///
/// This is a convenience shorthand for:
/// ```rust,ignore
/// fn_widget! { @SmoothLayout { $($t)* } }
/// ```
///
/// # Example
/// ```rust,ignore
/// smooth_layout! {
///   axes: SmoothAxes::SIZE,
///   init_size: Size::splat(0f32.into()),
///   @MyWidget {}
/// }
/// ```
#[macro_export]
macro_rules! smooth_layout {
  ($($t: tt)*) => { fn_widget! { @SmoothLayout { $($t)* } } };
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

/// Tracks the three mutually-exclusive animation phases.
///
/// - `Idle`: no animation running.
/// - `Pending`: animation was initiated this frame; `started_at` will be set to
///   `Instant::now()` in the next `once_before_layout` callback so the first
///   interpolated frame starts from elapsed ≈ 0.
/// - `Running(Instant)`: actively animating since the recorded start time.
#[derive(Clone, Copy, Default)]
enum AnimPhase {
  #[default]
  Idle,
  Pending,
  Running(Instant),
}

/// State shared via `Rc` between `SmoothLayout` and `once_before_layout`
/// closures. Contains only what the closures actually need.
#[derive(Default)]
struct SharedAnimState {
  phase: Cell<AnimPhase>,
  scheduled: Cell<bool>,
}

impl SmoothLayout {
  fn has_pos_axes(&self) -> bool { self.axes.intersects(SmoothAxes::POS) }

  fn has_size_axes(&self) -> bool { self.axes.intersects(SmoothAxes::SIZE) }

  /// Compute the current interpolated geometry.
  ///
  /// Non-selected axes always return the target value.
  fn current(&self) -> Rect {
    let factor = match self.anim.phase.get() {
      AnimPhase::Idle => return self.target.get(),
      // Animation just started this frame — use factor 0 to guarantee
      // deterministic first-frame output at exact `from`.
      AnimPhase::Pending => 0.0,
      AnimPhase::Running(started) => self
        .transition
        .rate_of_change(Instant::now() - started)
        .value(),
    };
    lerp_rect_axes(&self.from.get(), &self.target.get(), factor, self.axes)
  }

  fn is_animating(&self) -> bool {
    match self.anim.phase.get() {
      AnimPhase::Idle => false,
      AnimPhase::Pending => true,
      AnimPhase::Running(started) => !self
        .transition
        .rate_of_change(Instant::now() - started)
        .is_finish(),
    }
  }

  fn target_size(&self) -> Size { self.target.get().size }

  fn animated_size(&self) -> Size { self.current().size }

  fn layout_size(&self) -> Size {
    match self.layout_impact {
      LayoutImpact::NoLayout => self.target_size(),
      LayoutImpact::SelfLayout | LayoutImpact::SubtreeLayout => self.animated_size(),
    }
  }

  fn basis_size(&self) -> Size {
    match self.layout_impact {
      LayoutImpact::SubtreeLayout => self.animated_size(),
      LayoutImpact::NoLayout | LayoutImpact::SelfLayout => self.target_size(),
    }
  }

  fn layout_pos(&self) -> Point { self.current().origin }

  fn set_target_origin(&self, origin: Point) {
    let mut t = self.target.get();
    t.origin = origin;
    self.target.set(t);
  }

  fn set_target_size(&self, size: Size) {
    let mut t = self.target.get();
    t.size = size;
    self.target.set(t);
  }

  fn set_from_origin(&self, origin: Point) {
    let mut t = self.from.get();
    t.origin = origin;
    self.from.set(t);
  }

  fn set_from_size(&self, size: Size) {
    let mut t = self.from.get();
    t.size = size;
    self.from.set(t);
  }

  fn scale_factor(&self) -> Vector {
    let basis = self.basis_size();
    let animated = self.animated_size();

    let sx = if basis.width > 0. { animated.width / basis.width } else { 1. };
    let sy = if basis.height > 0. { animated.height / basis.height } else { 1. };
    Vector::new(sx, sy)
  }

  fn required_dirty_phase(&self) -> DirtyPhase {
    if !self.has_size_axes() && self.has_pos_axes() {
      DirtyPhase::Position
    } else if self.has_size_axes()
      && self.layout_impact == LayoutImpact::NoLayout
      && !self.has_pos_axes()
    {
      DirtyPhase::Paint
    } else {
      DirtyPhase::Layout
    }
  }

  fn is_content_motion_active(&self, animations_on: bool) -> bool {
    self.has_size_axes() && animations_on && self.is_animating()
  }

  fn resolve_init_size(&self, clamp_max: Size, fallback: Size) -> Option<Size> {
    if !self.has_size_axes() {
      return None;
    }

    let mut width = fallback.width;
    let mut height = fallback.height;
    let mut has_init = false;

    if self.axes.contains(SmoothAxes::W)
      && let Some(v) = self.init_width
    {
      width = v.into_pixel(clamp_max.width);
      has_init = true;
    }

    if self.axes.contains(SmoothAxes::H)
      && let Some(v) = self.init_height
    {
      height = v.into_pixel(clamp_max.height);
      has_init = true;
    }

    has_init.then_some(Size::new(width, height))
  }

  fn resolve_init_pos(&self, size: Size, clamp: BoxClamp, fallback: Point) -> Option<Point> {
    let mut x = fallback.x;
    let mut y = fallback.y;
    let mut has_init = false;

    let max = Size::new(clamp.container_width(size.width), clamp.container_height(size.height));

    if self.axes.contains(SmoothAxes::X)
      && let Some(anchor) = &self.init_pos.x
    {
      x = anchor.calculate(max.width, size.width);
      has_init = true;
    }

    if self.axes.contains(SmoothAxes::Y)
      && let Some(anchor) = &self.init_pos.y
    {
      y = anchor.calculate(max.height, size.height);
      has_init = true;
    }

    has_init.then_some(Point::new(x, y))
  }
}

// --- Animation control ---

impl SmoothLayout {
  /// Mark animation as pending.
  fn begin_animation(&self) { self.anim.phase.set(AnimPhase::Pending); }

  /// Stop the animation if it has finished.
  fn stop_if_idle(&self) {
    if self.is_animating() {
      return;
    }
    self.anim.phase.set(AnimPhase::Idle);
  }

  /// Schedule dirty marking for the next animation frame via
  /// `once_before_layout`.
  ///
  /// The callback self-chains while animation is running, so frame progression
  /// does not depend on layout/position hooks being triggered.
  fn schedule_animation_frame(
    &self, window: &Rc<Window>, widget_id: WidgetId, marker: &crate::widget_tree::DirtyMarker,
  ) {
    if self.anim.scheduled.get() {
      return;
    }
    Self::schedule_animation_frame_inner(
      window.clone(),
      widget_id,
      marker.clone(),
      self.anim.clone(),
      self.transition.dyn_clone(),
      self.required_dirty_phase(),
    );
  }

  fn schedule_animation_frame_inner(
    window: Rc<Window>, widget_id: WidgetId, marker: crate::widget_tree::DirtyMarker,
    anim: Rc<SharedAnimState>, transition: Box<dyn Transition>, dirty: DirtyPhase,
  ) {
    anim.scheduled.set(true);
    let wnd = window.clone();
    window.once_before_layout(move || {
      anim.scheduled.set(false);

      // Transition from pending to active: set the real start time now so
      // the first interpolated frame starts from elapsed ≈ 0.
      if matches!(anim.phase.get(), AnimPhase::Pending) {
        anim.phase.set(AnimPhase::Running(Instant::now()));
      }

      let animating = if let AnimPhase::Running(started) = anim.phase.get() {
        !transition
          .rate_of_change(Instant::now() - started)
          .is_finish()
      } else {
        false
      };

      // Mark dirty for one more frame (either continuing animation or
      // settling at exact target on the final frame).
      marker.mark(widget_id, dirty);

      if animating {
        Self::schedule_animation_frame_inner(wnd, widget_id, marker, anim, transition, dirty);
      } else {
        anim.phase.set(AnimPhase::Idle);
      }
    });
  }

  fn update_target_size(
    &self, widget_id: WidgetId, target_size: Size, clamp_max: Size, animations_on: bool,
    window: &Rc<Window>, marker: &crate::widget_tree::DirtyMarker,
  ) {
    let size_changed = self.target.get().size != target_size;
    // Capture current visual position BEFORE updating target: `current()` falls
    // back to `target` when no animation is running, so it must be read first.
    let current = self.current();
    self.set_target_size(target_size);

    if !animations_on || !self.has_size_axes() {
      self.stop_if_idle();
      return;
    }

    if !self.layout_settled.get() {
      // First layout: start from init value.
      let from_size = self
        .resolve_init_size(clamp_max, target_size)
        .unwrap_or(target_size);
      self.set_from_size(from_size);
      if from_size != target_size {
        self.begin_animation();
      }
    } else if size_changed {
      // Retarget from current visual position.
      self.from.set(current);
      self.begin_animation();
    }

    if self.is_animating() {
      self.schedule_animation_frame(window, widget_id, marker);
    }
  }

  #[allow(clippy::too_many_arguments)]
  fn update_target_pos(
    &self, widget_id: WidgetId, target_pos: Point, size: Size, clamp: BoxClamp,
    animations_on: bool, window: &Rc<Window>, marker: &crate::widget_tree::DirtyMarker,
  ) {
    let pos_changed = self.target.get().origin != target_pos;
    // Capture current visual position BEFORE updating target: `current()` falls
    // back to `target` when no animation is running, so it must be read first.
    let current = self.current();
    self.set_target_origin(target_pos);

    if !animations_on || !self.has_pos_axes() {
      self.stop_if_idle();
      self.layout_settled.set(true);
      return;
    }

    if !self.layout_settled.get() {
      let from_pos = self
        .resolve_init_pos(size, clamp, target_pos)
        .unwrap_or(target_pos);
      self.set_from_origin(from_pos);
      if from_pos != target_pos {
        self.begin_animation();
      }
      self.layout_settled.set(true);
    } else if pos_changed {
      // Retarget from current visual position.
      self.from.set(current);
      self.begin_animation();
    }

    if self.is_animating() {
      self.schedule_animation_frame(window, widget_id, marker);
    }
  }
}

impl Drop for SmoothLayout {
  fn drop(&mut self) {
    // Set phase = Idle so that any still-queued `once_before_layout` closures
    // (which share `anim` via `Rc`) see `phase = Idle` and stop the chain.
    self.anim.phase.set(AnimPhase::Idle);
  }
}

fn point_in_size(pos: Point, size: Size) -> bool {
  pos.x >= 0. && pos.y >= 0. && pos.x <= size.width && pos.y <= size.height
}

fn apply_size_axes_to_clamp(clamp: &mut BoxClamp, axes: SmoothAxes, size: Size) {
  if axes.contains(SmoothAxes::W) {
    clamp.min.width = size.width;
    clamp.max.width = size.width;
  }

  if axes.contains(SmoothAxes::H) {
    clamp.min.height = size.height;
    clamp.max.height = size.height;
  }
}

fn combine_dirty_phase(wrapper: DirtyPhase, host: DirtyPhase) -> DirtyPhase {
  use DirtyPhase::*;

  match (wrapper, host) {
    (LayoutSubtree, _) | (_, LayoutSubtree) => LayoutSubtree,
    (Layout, _) | (_, Layout) => Layout,
    (Position, _) | (_, Position) => Position,
    (Paint, Paint) => Paint,
  }
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
    let widget_id = ctx.widget_id();
    let window = ctx.window();
    let animations_on = animations_enabled(&window);
    let marker = ctx.tree.dirty_marker();

    let target_size = host.measure(clamp, ctx);
    self.update_target_size(widget_id, target_size, clamp.max, animations_on, &window, &marker);

    let layout_size = self.layout_size();

    if self.has_size_axes()
      && self.layout_impact == LayoutImpact::SubtreeLayout
      && animations_on
      && layout_size != target_size
    {
      let mut smooth_clamp = clamp;
      apply_size_axes_to_clamp(&mut smooth_clamp, self.axes, layout_size);
      host.measure(smooth_clamp, ctx)
    } else {
      layout_size
    }
  }

  fn place_children(&self, size: Size, host: &dyn Render, ctx: &mut PlaceCtx) {
    if self.has_size_axes() && self.layout_impact != LayoutImpact::SubtreeLayout {
      host.place_children(self.basis_size(), ctx)
    } else {
      host.place_children(size, ctx)
    }
  }

  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    let widget_id = ctx.widget_id();
    let target_pos = host.adjust_position(pos, ctx);
    let window = ctx.window();
    let animations_on = animations_enabled(&window);
    let size = ctx.widget_box_size(widget_id).unwrap_or_default();
    let clamp = ctx.clamp();
    let marker = ctx.tree.dirty_marker();
    let first = !self.layout_settled.get();

    // When both pos and size axes are active and past the first layout,
    // position follows target directly -- size animation already accounts for
    // the visual shift.
    if animations_on && self.has_size_axes() && self.has_pos_axes() && !first {
      if self.target.get().origin != target_pos {
        self.set_target_origin(target_pos);
      }
      return target_pos;
    }

    self.update_target_pos(widget_id, target_pos, size, clamp, animations_on, &window, &marker);
    self.layout_pos()
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.is_content_motion_active(animations_enabled(&ctx.window())) {
      match self.content_motion {
        ContentMotion::ClipReveal => {
          let rect = Rect::from_size(self.animated_size());
          ctx.box_painter().clip(Path::rect(&rect).into());
        }
        ContentMotion::Scale => {
          let scale = self.scale_factor();
          if scale != Vector::one() {
            ctx.painter().scale(scale.x, scale.y);
          }
        }
      }
    }

    host.paint(ctx)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if !self.is_content_motion_active(animations_enabled(&ctx.window())) {
      return host.hit_test(ctx, pos);
    }

    let box_pos = ctx.box_pos().unwrap_or(Point::zero());
    let local_pos = pos - box_pos.to_vector();
    let animated_size = self.animated_size();
    let scale = self.scale_factor();

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
    // Scale transform applies whenever the scale animation is geometrically
    // active, regardless of the window ANIMATIONS flag.
    if self.content_motion != ContentMotion::Scale || !self.has_size_axes() || !self.is_animating()
    {
      return host.get_transform();
    }

    let scale = self.scale_factor();
    if scale == Vector::one() {
      return host.get_transform();
    }

    let t = Transform::scale(scale.x, scale.y);
    if let Some(host_t) = host.get_transform() { Some(t.then(&host_t)) } else { Some(t) }
  }

  fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase {
    combine_dirty_phase(self.wrapper_dirty_phase(), host.dirty_phase())
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { self.required_dirty_phase() }
}

fn default_transition() -> Box<dyn Transition> {
  Box::new(EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(200) })
}

/// Builder for [`SmoothLayout`].
///
/// Created via [`SmoothLayout::declarer`] and normally used implicitly through
/// the `@SmoothLayout { … }` or [`smooth_layout!`] DSL.
#[derive(Default)]
pub struct SmoothLayoutDeclarer {
  transition: Option<Box<dyn Transition>>,
  axes: SmoothAxes,
  layout_impact: LayoutImpact,
  content_motion: ContentMotion,
  init_pos: Anchor,
  init_width: Option<Measure>,
  init_height: Option<Measure>,
  fat_obj: FatObj<()>,
}

impl SmoothLayoutDeclarer {
  /// Set the transition used to interpolate geometry.
  ///
  /// Defaults to a 200 ms linear ease when not specified.
  pub fn with_transition(&mut self, transition: impl Transition + 'static) -> &mut Self {
    self.transition = Some(Box::new(transition));
    self
  }

  /// Set how the animated size is reported to the layout system.
  ///
  /// See [`LayoutImpact`] for available options. Defaults to
  /// [`LayoutImpact::SelfLayout`].
  pub fn with_layout_impact(&mut self, impact: LayoutImpact) -> &mut Self {
    self.layout_impact = impact;
    self
  }

  /// Set the visual effect applied to content during a size animation.
  ///
  /// See [`ContentMotion`] for available options. Defaults to
  /// [`ContentMotion::ClipReveal`].
  pub fn with_content_motion(&mut self, motion: ContentMotion) -> &mut Self {
    self.content_motion = motion;
    self
  }

  /// Restrict animation to the specified axes.
  ///
  /// Defaults to [`SmoothAxes::ALL`].
  pub fn with_axes(&mut self, axes: SmoothAxes) -> &mut Self {
    self.axes = axes;
    self
  }

  /// Set the initial position (both X and Y) for the entry animation.
  ///
  /// Only used on first appearance. If not set, the widget starts at its
  /// target position (no entry animation).
  pub fn with_init_pos(&mut self, init: impl Into<Anchor>) -> &mut Self {
    self.init_pos = init.into();
    self
  }

  /// Set the initial size (both width and height) for the entry animation.
  ///
  /// Accepts [`Measure`] values, which can be absolute pixels or percentages
  /// of the containing box. Only used on first appearance.
  pub fn with_init_size(&mut self, init: impl Into<Size<Measure>>) -> &mut Self {
    let init = init.into();
    self.init_width = Some(init.width);
    self.init_height = Some(init.height);
    self
  }

  /// Set the initial X position for the entry animation.
  pub fn with_init_x(&mut self, init: impl Into<AnchorX>) -> &mut Self {
    self.init_pos.x = Some(init.into());
    self
  }

  /// Set the initial Y position for the entry animation.
  pub fn with_init_y(&mut self, init: impl Into<AnchorY>) -> &mut Self {
    self.init_pos.y = Some(init.into());
    self
  }

  /// Set the initial width for the entry animation.
  ///
  /// Accepts absolute pixels (`5.0_f32.into()`) or a percentage
  /// (`50.percent()`). Only used on first appearance.
  pub fn with_init_width(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_width = Some(init.into());
    self
  }

  /// Set the initial height for the entry animation.
  ///
  /// Accepts absolute pixels (`5.0_f32.into()`) or a percentage
  /// (`50.percent()`). Only used on first appearance.
  pub fn with_init_height(&mut self, init: impl Into<Measure>) -> &mut Self {
    self.init_height = Some(init.into());
    self
  }
}

impl std::ops::Deref for SmoothLayoutDeclarer {
  type Target = FatObj<()>;
  fn deref(&self) -> &Self::Target { &self.fat_obj }
}

impl std::ops::DerefMut for SmoothLayoutDeclarer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.fat_obj }
}

impl Declare for SmoothLayout {
  type Builder = SmoothLayoutDeclarer;

  fn declarer() -> Self::Builder {
    SmoothLayoutDeclarer { axes: SmoothAxes::ALL, ..Default::default() }
  }
}

impl ObjDeclarer for SmoothLayoutDeclarer {
  type Target = FatObj<SmoothLayout>;

  fn finish(self) -> Self::Target {
    let Self {
      transition,
      axes,
      layout_impact,
      content_motion,
      init_pos,
      init_width,
      init_height,
      fat_obj,
    } = self;
    fat_obj.map(|_| SmoothLayout {
      axes,
      layout_impact,
      content_motion,
      transition: transition.unwrap_or_else(default_transition),
      init_pos,
      init_width,
      init_height,
      target: Cell::default(),
      from: Cell::default(),
      layout_settled: Cell::default(),
      anim: Rc::new(SharedAnimState::default()),
    })
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
      WidgetTester::new(crate::smooth_layout! {
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
      WidgetTester::new(crate::smooth_layout! {
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
      WidgetTester::new(crate::smooth_layout! {
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
  fn smooth_layout_no_layout_size_keeps_redrawing_while_animating() {
    reset_test_env!();

    const SHORT_TRANS: EasingTransition<LinearEasing> =
      EasingTransition { easing: easing::LinearEasing, duration: Duration::from_millis(120) };

    let mut wnd = TestWindow::new(
      fn_widget! {
        @SmoothLayout {
          axes: SmoothAxes::SIZE,
          layout_impact: LayoutImpact::NoLayout,
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
      "expected smooth no-layout size animation to redraw across multiple frames, \
       redraw_count={redraw_count}",
    );
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
