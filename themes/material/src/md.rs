//! This module defines the constant variables of the material theme that are
//! frequently used.
//!
//! See: https://m3.material.io

use ribir_core::prelude::*;

// The motion variables.
//
// Source:
// - https://m3.material.io/styles/motion/overview/how-it-works
// - https://unpkg.com/@material/web@nightly/tokens/versions/v30_0/sass/_md-sys-motion.scss
//
// Synced with Material token definitions on 2026-02-13.
pub mod motion {
  use ribir_core::prelude::{BuildCtx, Provider};

  /// Material theme motion model (theme-internal guidance)
  ///
  /// This module is the single source of truth for Material motion tokens in
  /// this theme.
  ///
  /// ## Design
  ///
  /// - `Scheme`: product-level motion style (`Expressive` / `Standard`)
  /// - `Speed`: token speed (`Fast` / `Default` / `Slow`)
  /// - `spring`: physics tokens and runtime facade
  /// - `duration`: legacy duration tokens (kept for compatibility)
  ///
  /// ## Provider model
  ///
  /// `current()` reads `Scheme` from `BuildCtx` providers. If no provider is
  /// present (or outside build context), the fallback is `Expressive`.
  ///
  /// - Provide scheme in subtree: `providers: [md::motion::provider(...)]`
  /// - Read active scheme: `md::motion::current()`
  ///
  /// ## Spring usage rules
  ///
  /// - Spatial properties (position/size/shape/rotation): use spatial spring
  /// - Effects properties (opacity/color): use effects spring
  /// - Prefer `spring::{spatial,effects}` over hand-picking raw numbers
  ///
  /// ## Compatibility
  ///
  /// Existing `md::easing::*` and `md::easing::duration::*` are intentionally
  /// retained as a compatibility layer.
  ///
  /// ## Quick usage
  ///
  /// - Use current-scheme spring transitions:
  ///   - `md::motion::spring::spatial::default()`
  ///   - `md::motion::spring::effects::fast()`
  /// - Use explicit scheme tokens when needed:
  ///   - `md::motion::spring::scheme::expressive::*`
  ///   - `md::motion::spring::scheme::standard::*`

  /// Motion schemes from Material Expressive.
  ///
  /// Expressive is the default scheme and should be used for most products.
  /// Standard is a more restrained alternative for utilitarian surfaces.
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
  pub enum Scheme {
    #[default]
    Expressive,
    Standard,
  }

  /// Spring speed buckets used by Material motion tokens.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Speed {
    Fast,
    Default,
    Slow,
  }

  /// The global/default scheme used by Material theme tokens.
  ///
  /// If no provider is present (or we're outside build context), fallback to
  /// Expressive.
  pub fn current() -> Scheme {
    BuildCtx::try_get()
      .and_then(|ctx| Provider::of::<Scheme>(ctx).map(|s| *s))
      .unwrap_or(Scheme::Expressive)
  }

  /// Helper to provide a motion scheme in a subtree.
  pub fn provider(scheme: Scheme) -> Provider { Provider::new(scheme) }

  pub mod duration {
    use crate::Duration;

    // ## Short duration
    // These are used for small utility-focused transitions.
    pub const SHORT1: Duration = Duration::from_millis(50);
    pub const SHORT2: Duration = Duration::from_millis(100);
    pub const SHORT3: Duration = Duration::from_millis(150);
    pub const SHORT4: Duration = Duration::from_millis(200);

    // ## Medium durations
    // These are used for transitions that traverse a medium area of the screen.
    pub const MEDIUM1: Duration = Duration::from_millis(250);
    pub const MEDIUM2: Duration = Duration::from_millis(300);
    pub const MEDIUM3: Duration = Duration::from_millis(350);
    pub const MEDIUM4: Duration = Duration::from_millis(400);

    // ## Long durations
    // These durations are often paired with Emphasized easing. They're used for
    // large expressive transitions.
    pub const LONG1: Duration = Duration::from_millis(450);
    pub const LONG2: Duration = Duration::from_millis(500);
    pub const LONG3: Duration = Duration::from_millis(550);
    pub const LONG4: Duration = Duration::from_millis(600);

    // ## Extra long durations
    // Though rare, some transitions use durations above 600ms. These are usually
    // used for ambient transitions that don't involve user input.
    pub const EXTRA_LONG1: Duration = Duration::from_millis(700);
    pub const EXTRA_LONG2: Duration = Duration::from_millis(800);
    pub const EXTRA_LONG3: Duration = Duration::from_millis(900);
    pub const EXTRA_LONG4: Duration = Duration::from_millis(1000);

    #[deprecated(note = "use EXTRA_LONG1")]
    pub const EXR_LONG1: Duration = EXTRA_LONG1;
    #[deprecated(note = "use EXTRA_LONG2")]
    pub const EXR_LONG2: Duration = EXTRA_LONG2;
    #[deprecated(note = "use EXTRA_LONG3")]
    pub const EXR_LONG3: Duration = EXTRA_LONG3;
    #[deprecated(note = "use EXTRA_LONG4")]
    pub const EXR_LONG4: Duration = EXTRA_LONG4;
  }

  pub mod easing {
    use ribir_core::animation::easing::CubicBezierEasing;
    pub use ribir_core::animation::easing::LINEAR;

    // The Emphasized easing set is recommended for most transitions to capture
    // the style of M3.
    pub const EMPHASIZED: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
    pub const EMPHASIZED_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 0.8, 0.15);
    pub const EMPHASIZED_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0.05, 0.7, 0.1, 1.);

    // The Standard easing set can be used for small utility-focused
    // transitions that need to be quick.
    pub const STANDARD: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
    pub const STANDARD_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 1., 1.);
    pub const STANDARD_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0., 0., 0., 1.);

    // Legacy easing tokens are kept for interop with older M3 motion sets.
    pub const LEGACY: CubicBezierEasing = CubicBezierEasing::new(0.4, 0., 0.2, 1.);
    pub const LEGACY_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.4, 0., 1., 1.);
    pub const LEGACY_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0., 0., 0.2, 1.);
  }

  pub mod spring {
    use ribir_core::animation::{AnimateProgress, Transition};

    use super::{Scheme, Speed};
    use crate::Duration;

    /// Spring token system for Material motion.
    ///
    /// Organization:
    /// - `scheme::{expressive, standard}`: canonical per-scheme token values
    /// - `duration::{expressive, standard}`: scheme-specific spring durations
    /// - `{spatial,effects}`: runtime facade based on current scheme

    pub mod duration {
      pub mod expressive {
        use crate::Duration;

        pub const FAST_SPATIAL: Duration = Duration::from_millis(350);
        pub const DEFAULT_SPATIAL: Duration = Duration::from_millis(500);
        pub const SLOW_SPATIAL: Duration = Duration::from_millis(650);

        pub const FAST_EFFECTS: Duration = Duration::from_millis(150);
        pub const DEFAULT_EFFECTS: Duration = Duration::from_millis(200);
        pub const SLOW_EFFECTS: Duration = Duration::from_millis(300);
      }

      pub mod standard {
        use crate::Duration;

        pub const FAST_SPATIAL: Duration = Duration::from_millis(350);
        pub const DEFAULT_SPATIAL: Duration = Duration::from_millis(500);
        pub const SLOW_SPATIAL: Duration = Duration::from_millis(750);

        pub const FAST_EFFECTS: Duration = Duration::from_millis(150);
        pub const DEFAULT_EFFECTS: Duration = Duration::from_millis(200);
        pub const SLOW_EFFECTS: Duration = Duration::from_millis(300);
      }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct SpringPairToken {
      pub damping: f32,
      pub stiffness: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct SpringToken {
      pub effects: SpringPairToken,
      pub spatial: SpringPairToken,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct SpringSchemeToken {
      pub fast: SpringToken,
      pub default: SpringToken,
      pub slow: SpringToken,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct SpringTransition {
      pub duration: Duration,
      pub damping: f32,
      pub stiffness: f32,
      pub settle_tolerance: f32,
    }

    impl SpringPairToken {
      pub const fn new(damping: f32, stiffness: f32) -> Self { Self { damping, stiffness } }

      pub fn transition(self, duration: Duration) -> SpringTransition {
        SpringTransition::new(duration, self.damping, self.stiffness)
      }
    }

    impl SpringToken {
      pub const fn new(effects: SpringPairToken, spatial: SpringPairToken) -> Self {
        Self { effects, spatial }
      }

      pub fn effects_transition(self, duration: Duration) -> SpringTransition {
        self.effects.transition(duration)
      }

      pub fn spatial_transition(self, duration: Duration) -> SpringTransition {
        self.spatial.transition(duration)
      }
    }

    impl SpringSchemeToken {
      pub const fn new(fast: SpringToken, default: SpringToken, slow: SpringToken) -> Self {
        Self { fast, default, slow }
      }

      pub const fn speed(self, speed: Speed) -> SpringToken {
        match speed {
          Speed::Fast => self.fast,
          Speed::Default => self.default,
          Speed::Slow => self.slow,
        }
      }

      pub fn spatial_transition(self, speed: Speed, duration: Duration) -> SpringTransition {
        self.speed(speed).spatial_transition(duration)
      }

      pub fn effects_transition(self, speed: Speed, duration: Duration) -> SpringTransition {
        self.speed(speed).effects_transition(duration)
      }
    }

    impl SpringTransition {
      pub fn new(duration: Duration, damping: f32, stiffness: f32) -> Self {
        Self { duration, damping, stiffness, settle_tolerance: 0.001 }
      }

      pub fn with_settle_tolerance(mut self, settle_tolerance: f32) -> Self {
        self.settle_tolerance = settle_tolerance.max(0.);
        self
      }

      fn response_at(&self, t: f32) -> f32 {
        let damping = self.damping.max(0.0001);
        let omega = self.stiffness.max(1.).sqrt();

        if damping < 1. {
          let d = (1. - damping * damping).sqrt();
          let wd = omega * d;
          let exp = (-damping * omega * t).exp();
          let sin = (wd * t).sin();
          let cos = (wd * t).cos();
          1. - exp * (cos + damping / d * sin)
        } else if (damping - 1.).abs() < f32::EPSILON {
          1. - (-omega * t).exp() * (1. + omega * t)
        } else {
          let d = (damping * damping - 1.).sqrt();
          let r1 = -omega * (damping - d);
          let r2 = -omega * (damping + d);
          let c2 = r1 / (r1 - r2);
          let c1 = 1. - c2;
          1. - c1 * (r1 * t).exp() - c2 * (r2 * t).exp()
        }
      }
    }

    impl Transition for SpringTransition {
      fn rate_of_change(&self, run_dur: Duration) -> AnimateProgress {
        if self.duration.is_zero() || run_dur >= self.duration {
          return AnimateProgress::Finish;
        }

        let t = run_dur.as_secs_f32() / self.duration.as_secs_f32();
        let progress = self.response_at(t.clamp(0., 1.));

        if !progress.is_finite() {
          return AnimateProgress::Finish;
        }

        if (1. - progress).abs() <= self.settle_tolerance {
          AnimateProgress::Finish
        } else {
          AnimateProgress::Between(progress)
        }
      }

      fn duration(&self) -> Duration { self.duration }

      fn dyn_clone(&self) -> Box<dyn Transition> { Box::new(*self) }
    }

    /// Scheme-organized spring token definitions.
    pub mod scheme {
      use super::{SpringPairToken, SpringSchemeToken, SpringToken};

      pub mod expressive {
        use super::{SpringPairToken, SpringSchemeToken, SpringToken};

        pub const FAST: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 3800.), SpringPairToken::new(0.9, 1400.));

        pub const DEFAULT: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 1600.), SpringPairToken::new(0.9, 700.));

        // Reserved for large/full-screen transitions.
        pub const SLOW: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 800.), SpringPairToken::new(0.9, 300.));

        pub const ALL: SpringSchemeToken = SpringSchemeToken::new(FAST, DEFAULT, SLOW);
      }

      pub mod standard {
        use super::{SpringPairToken, SpringSchemeToken, SpringToken};

        pub const FAST: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 3800.), SpringPairToken::new(1., 1400.));

        pub const DEFAULT: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 1600.), SpringPairToken::new(1., 700.));

        pub const SLOW: SpringToken =
          SpringToken::new(SpringPairToken::new(1., 800.), SpringPairToken::new(1., 300.));

        pub const ALL: SpringSchemeToken = SpringSchemeToken::new(FAST, DEFAULT, SLOW);
      }
    }

    pub const fn of(scheme: Scheme) -> SpringSchemeToken {
      match scheme {
        Scheme::Expressive => scheme::expressive::ALL,
        Scheme::Standard => scheme::standard::ALL,
      }
    }

    pub fn current() -> SpringSchemeToken { of(super::current()) }

    fn spatial_duration(speed: Speed) -> Duration {
      match super::current() {
        Scheme::Expressive => match speed {
          Speed::Fast => duration::expressive::FAST_SPATIAL,
          Speed::Default => duration::expressive::DEFAULT_SPATIAL,
          Speed::Slow => duration::expressive::SLOW_SPATIAL,
        },
        Scheme::Standard => match speed {
          Speed::Fast => duration::standard::FAST_SPATIAL,
          Speed::Default => duration::standard::DEFAULT_SPATIAL,
          Speed::Slow => duration::standard::SLOW_SPATIAL,
        },
      }
    }

    fn effects_duration(speed: Speed) -> Duration {
      match super::current() {
        Scheme::Expressive => match speed {
          Speed::Fast => duration::expressive::FAST_EFFECTS,
          Speed::Default => duration::expressive::DEFAULT_EFFECTS,
          Speed::Slow => duration::expressive::SLOW_EFFECTS,
        },
        Scheme::Standard => match speed {
          Speed::Fast => duration::standard::FAST_EFFECTS,
          Speed::Default => duration::standard::DEFAULT_EFFECTS,
          Speed::Slow => duration::standard::SLOW_EFFECTS,
        },
      }
    }

    pub mod spatial {
      use super::{Duration, Speed, SpringTransition, current, spatial_duration};

      pub fn fast() -> SpringTransition {
        with_duration(spatial_duration(Speed::Fast), Speed::Fast)
      }

      pub fn default() -> SpringTransition {
        with_duration(spatial_duration(Speed::Default), Speed::Default)
      }

      pub fn slow() -> SpringTransition {
        with_duration(spatial_duration(Speed::Slow), Speed::Slow)
      }

      pub fn with_duration(duration: Duration, speed: Speed) -> SpringTransition {
        current().spatial_transition(speed, duration)
      }
    }

    pub mod effects {
      use super::{Duration, Speed, SpringTransition, current, effects_duration};

      pub fn fast() -> SpringTransition {
        with_duration(effects_duration(Speed::Fast), Speed::Fast)
      }

      pub fn default() -> SpringTransition {
        with_duration(effects_duration(Speed::Default), Speed::Default)
      }

      pub fn slow() -> SpringTransition {
        with_duration(effects_duration(Speed::Slow), Speed::Slow)
      }

      pub fn with_duration(duration: Duration, speed: Speed) -> SpringTransition {
        current().effects_transition(speed, duration)
      }
    }
  }

  pub mod scheme {
    pub use super::Scheme::{Expressive as EXPRESSIVE, Standard as STANDARD};
  }
}

// Compatibility layer for existing call sites.
pub mod easing {
  pub use super::motion::easing::*;

  pub mod duration {
    pub use super::super::motion::duration::*;
  }
}

// These are the constant variables commonly used in the material theme.

pub const RADIUS_5: Radius = Radius::all(5.);
pub const RADIUS_10: Radius = Radius::all(10.);
pub const RADIUS_12: Radius = Radius::all(12.);
pub const RADIUS_16: Radius = Radius::all(16.);
pub const RADIUS_20: Radius = Radius::all(20.);

pub const RADIUS_1: Radius = Radius::all(1.);
pub const RADIUS_2: Radius = Radius::all(2.);
pub const RADIUS_4: Radius = Radius::all(4.);
pub const RADIUS_8: Radius = Radius::all(8.);

pub const THICKNESS_4: f32 = 4.;
pub const THICKNESS_8: f32 = 8.;

pub const SIZE_10: Size = Size::new(10., 10.);
pub const SIZE_20: Size = Size::new(20., 20.);
pub const SIZE_40: Size = Size::new(40., 40.);

pub const SIZE_4: Size = Size::new(4., 4.);
pub const SIZE_8: Size = Size::new(8., 8.);
pub const SIZE_18: Size = Size::new(18., 18.);
pub const SIZE_24: Size = Size::new(24., 24.);
pub const SIZE_36: Size = Size::new(36., 36.);
pub const SIZE_48: Size = Size::new(48., 48.);
pub const SIZE_64: Size = Size::new(64., 64.);

pub const EDGES_2: EdgeInsets = EdgeInsets::all(2.);
pub const EDGES_HOR_2: EdgeInsets = EdgeInsets::horizontal(2.);
pub const EDGES_VER_2: EdgeInsets = EdgeInsets::vertical(2.);

pub const EDGES_4: EdgeInsets = EdgeInsets::all(4.);
pub const EDGES_HOR_4: EdgeInsets = EdgeInsets::horizontal(4.);
pub const EDGES_VER_4: EdgeInsets = EdgeInsets::vertical(4.);
pub const EDGES_LEFT_4: EdgeInsets = EdgeInsets::only_left(4.);
pub const EDGES_RIGHT_4: EdgeInsets = EdgeInsets::only_right(4.);
pub const EDGES_TOP_4: EdgeInsets = EdgeInsets::only_top(4.);
pub const EDGES_BOTTOM_4: EdgeInsets = EdgeInsets::only_bottom(4.);

pub const EDGES_HOR_6: EdgeInsets = EdgeInsets::horizontal(6.);

pub const EDGES_8: EdgeInsets = EdgeInsets::all(8.);
pub const EDGES_HOR_8: EdgeInsets = EdgeInsets::horizontal(8.);
pub const EDGES_VER_8: EdgeInsets = EdgeInsets::vertical(8.);
pub const EDGES_LEFT_8: EdgeInsets = EdgeInsets::only_left(8.);
pub const EDGES_RIGHT_8: EdgeInsets = EdgeInsets::only_right(8.);
pub const EDGES_TOP_8: EdgeInsets = EdgeInsets::only_top(8.);
pub const EDGES_BOTTOM_8: EdgeInsets = EdgeInsets::only_bottom(8.);

pub const EDGES_10: EdgeInsets = EdgeInsets::all(10.);
pub const EDGES_VER_10: EdgeInsets = EdgeInsets::vertical(10.);
pub const EDGES_TOP_10: EdgeInsets = EdgeInsets::only_top(10.);
pub const EDGES_BOTTOM_10: EdgeInsets = EdgeInsets::only_bottom(10.);

pub const EDGES_HOR_12: EdgeInsets = EdgeInsets::horizontal(12.);
pub const EDGES_VER_12: EdgeInsets = EdgeInsets::vertical(12.);

pub const EDGES_VER_14: EdgeInsets = EdgeInsets::vertical(14.);

pub const EDGES_16: EdgeInsets = EdgeInsets::all(16.);
pub const EDGES_HOR_16: EdgeInsets = EdgeInsets::horizontal(16.);
pub const EDGES_VER_16: EdgeInsets = EdgeInsets::vertical(16.);
pub const EDGES_LEFT_16: EdgeInsets = EdgeInsets::only_left(16.);
pub const EDGES_RIGHT_16: EdgeInsets = EdgeInsets::only_right(16.);
pub const EDGES_TOP_16: EdgeInsets = EdgeInsets::only_top(16.);
pub const EDGES_BOTTOM_16: EdgeInsets = EdgeInsets::only_bottom(16.);

pub const EDGES_HOR_24: EdgeInsets = EdgeInsets::horizontal(24.);
pub const EDGES_LEFT_24: EdgeInsets = EdgeInsets::only_left(24.);

pub const EDGES_HOR_32: EdgeInsets = EdgeInsets::horizontal(32.);
pub const EDGES_HOR_36: EdgeInsets = EdgeInsets::horizontal(36.);
pub const EDGES_HOR_48: EdgeInsets = EdgeInsets::horizontal(48.);

// Borders
pub fn border_1(color: Color) -> Border { Border::all(BorderSide::new(1., color.into())) }

pub fn border_1_top(color: Color) -> Border { Border::only_top(BorderSide::new(1., color.into())) }

pub fn border_1_right(color: Color) -> Border {
  Border::only_right(BorderSide::new(1., color.into()))
}
pub fn border_1_bottom(color: Color) -> Border {
  Border::only_bottom(BorderSide::new(1., color.into()))
}
pub fn border_1_left(color: Color) -> Border {
  Border::only_left(BorderSide::new(1., color.into()))
}

pub fn border_2() -> VariantMap<Variant<Color>, impl Fn(&Color) -> Border> {
  BuildCtx::color().map(|color| Border::all(BorderSide::new(2., (*color).into())))
}
pub fn border_2_surface_color() -> Border {
  let surface_variant = Palette::of(BuildCtx::get()).on_surface_variant();
  Border::all(BorderSide::new(2., surface_variant.into()))
}

/// M3 elevation shadow for a given level (0-5).
///
/// See: https://m3.material.io/styles/elevation/overview
///
/// - Level 0: 0dp (no shadow)
/// - Level 1: 1dp
/// - Level 2: 3dp
/// - Level 3: 6dp (FAB resting)
/// - Level 4: 8dp (FAB hover)
/// - Level 5: 12dp
pub fn elevation_shadow(level: u8, shadow_color: Color) -> BoxShadow {
  // M3 elevation dp values and alpha (higher elevation = slightly more visible
  // shadow)
  let (offset_y, blur, spread, alpha) = match level {
    0 => (0., 0., 0., 0.),
    1 => (1., 2., 0., 0.10),
    2 => (2., 4., 0., 0.12),
    3 => (3., 6., 1., 0.15),
    4 => (4., 8., 1., 0.18),
    _ => (6., 12., 2., 0.22), // Level 5+
  };
  BoxShadow::new(Point::new(0., offset_y), blur, spread, shadow_color.with_alpha(alpha as f32))
}
