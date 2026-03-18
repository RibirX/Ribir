//! Keyframes are used to manage the intermediate steps of animation states.
//!
//! This offers a straightforward method to define the state value and ensure
//! seamless animation transitions between the keyframes.
//!
//! You can employ the `keyframes!` macro to generate an animated state.
//!
//! Duplicate rates are a supported part of the API. Frames are sorted with a
//! **stable** ordering, so duplicate rates preserve declaration order. This
//! enables step-like jumps and loop hand-offs without relying on incidental
//! binary-search behavior.
//!
//! Note that a `100%` keyframe is part of the sampled timeline, but the final
//! animation value at `rate == 1.0` is still the animation's `to` value.
//!
//! # Example
//!
//! As the `keyframes!` macro returns a standard animated state, you can
//! smoothly transition through every change in the state.
//!
//! ```
//! use ribir::prelude::*;
//!
//! let _w = fn_widget! {
//!   let mut color_rect = @Container {
//!     size: Size::new(100., 100.),
//!     background: Color::RED,
//!   };
//!
//!   keyframes! {
//!     state: color_rect.background(),
//!     0.2 => Color::YELLOW.into(),
//!     0.5 => Color::BLUE.into(),
//!     0.8 => Color::GREEN.into(),
//!   }
//!   .transition(EasingTransition {
//!     duration: Duration::from_millis(1000),
//!     easing: easing::LinearEasing
//!   });
//!
//!   color_rect
//! };
//! ```
//!
//! Alternatively, you can utilize it to create an animation and control it.
//!
//! ```
//! use ribir::prelude::*;
//!
//! let _w = fn_widget! {
//!   let mut opacity_rect = @Container { size: Size::new(100., 100.) };
//!
//!   let animate = @Animate {
//!     state: keyframes! {
//!       state: opacity_rect.opacity(),
//!       20% => 0.,
//!       50% => 0.5,
//!       80% => 0.,
//!     },
//!     from: 0.,
//!     transition: EasingTransition {
//!       duration: Duration::from_millis(1000),
//!       easing: easing::LinearEasing
//!     }
//!   };
//!
//!   @(opacity_rect) {
//!     on_tap: move |_| animate.run()
//!   }
//! };
//! ```
//!
//! Duplicate-rate keyframes can be used to describe step effects or seamless
//! looping hand-offs.
//!
//! ```
//! use ribir::prelude::*;
//!
//! let opacity = Stateful::new(1.0_f32);
//! let _state = keyframes! {
//!   state: opacity,
//!   0% => 1.0,
//!   60% => 1.0,
//!   60% => 0.0,
//!   100% => 0.0,
//! };
//! ```
//!
//! Tuple state/value declarations are also supported directly by the macro and
//! are packed internally with [`animate_state_pack!`].
//!
//! ```
//! use ribir::prelude::*;
//!
//! let opacity = Stateful::new(0.0_f32);
//! let scale = Stateful::new(1.0_f32);
//! let _state = keyframes! {
//!   state: (opacity, scale),
//!   20% => (0.6, 1.05),
//!   60% => (1.0, 0.95),
//! };
//! ```

use std::fmt;

use ribir_core::animation::{AnimateState, CustomLerpState, Lerp};

#[derive(Debug)]
pub struct KeyFrames<S: AnimateState> {
  /// The state for the keyframes.
  pub state: S,
  /// Normalized keyframe stops. Duplicate rates are merged while preserving
  /// declaration order.
  stops: Box<[KeyFrameStop<S::Value>]>,
}

#[derive(Debug, Clone)]
pub struct KeyFrame<S> {
  pub rate: f32,
  pub state_value: S,
}

#[derive(Debug, Clone)]
struct KeyFrameStop<S> {
  rate: f32,
  in_value: S,
  out_value: S,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyFramesError {
  Empty,
  NonFiniteRate { index: usize, rate: f32 },
  OutOfRangeRate { index: usize, rate: f32 },
}

impl fmt::Display for KeyFramesError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      KeyFramesError::Empty => write!(f, "KeyFrames must contain at least one frame"),
      KeyFramesError::NonFiniteRate { index, rate } => {
        write!(f, "KeyFrames frame #{index} has a non-finite rate: {rate}")
      }
      KeyFramesError::OutOfRangeRate { index, rate } => {
        write!(f, "KeyFrames frame #{index} has an out-of-range rate: {rate}; expected 0..=1")
      }
    }
  }
}

impl<S: AnimateState> KeyFrames<S> {
  /// Creates a new `KeyFrames` instance with the given state and keyframes.
  ///
  /// # Arguments
  ///
  /// * `state` - The state for the keyframes.
  /// * `keyframes` - A vector of `KeyFrame` instances specifying the animation
  ///   frames.
  ///
  /// # Panics
  ///
  /// Panics if the frames are invalid.
  ///
  /// # Remarks
  ///
  /// The keyframes are sorted by their rate in ascending order with a stable
  /// sort so duplicate-rate declaration order becomes part of the timeline
  /// semantics. Do not replace this with `sort_unstable_by`.
  #[track_caller]
  pub fn new(state: S, keyframes: Vec<KeyFrame<S::Value>>) -> Self {
    Self::try_new(state, keyframes).unwrap_or_else(|err| panic!("Invalid KeyFrames: {err}"))
  }

  pub fn try_new(state: S, mut keyframes: Vec<KeyFrame<S::Value>>) -> Result<Self, KeyFramesError> {
    if keyframes.is_empty() {
      return Err(KeyFramesError::Empty);
    }

    for (index, frame) in keyframes.iter().enumerate() {
      if !frame.rate.is_finite() {
        return Err(KeyFramesError::NonFiniteRate { index, rate: frame.rate });
      }
      if !(0.0..=1.0).contains(&frame.rate) {
        return Err(KeyFramesError::OutOfRangeRate { index, rate: frame.rate });
      }
    }

    keyframes.sort_by(|a, b| a.rate.total_cmp(&b.rate));

    let mut stops: Vec<KeyFrameStop<S::Value>> = Vec::with_capacity(keyframes.len());
    for frame in keyframes {
      if let Some(stop) = stops.last_mut()
        && stop.rate == frame.rate
      {
        stop.out_value = frame.state_value;
      } else {
        stops.push(KeyFrameStop {
          rate: frame.rate,
          in_value: frame.state_value.clone(),
          out_value: frame.state_value,
        });
      }
    }

    Ok(Self { state, stops: stops.into_boxed_slice() })
  }

  fn sample_stops(
    stops: &[KeyFrameStop<S::Value>], from: &S::Value, to: &S::Value, rate: f32,
  ) -> S::Value
  where
    S::Value: Lerp,
  {
    debug_assert!(!stops.is_empty(), "KeyFrames invariant violated: stops must not be empty");

    if rate <= 0.0 {
      return stops
        .first()
        .filter(|stop| stop.rate == 0.0)
        .map_or_else(|| from.clone(), |stop| stop.out_value.clone());
    }

    if rate >= 1.0 {
      return to.clone();
    }

    let next_idx = stops.partition_point(|stop| stop.rate < rate);

    if let Some(stop) = stops.get(next_idx)
      && stop.rate == rate
    {
      return stop.out_value.clone();
    }

    let (prev_rate, prev_value) = stops
      .get(next_idx.wrapping_sub(1))
      .map_or((0.0, from), |prev| (prev.rate, &prev.out_value));

    let (next_rate, next_value) = stops
      .get(next_idx)
      .map_or((1.0, to), |next| (next.rate, &next.in_value));

    prev_value.lerp(next_value, (rate - prev_rate) / (next_rate - prev_rate))
  }

  #[allow(clippy::type_complexity)]
  /// Converts the `KeyFrames` into a `CustomLerpState` that can be used for
  /// animations.
  pub fn into_lerp_fn_state(
    self,
  ) -> CustomLerpState<S, impl FnMut(&S::Value, &S::Value, f32) -> S::Value + Clone>
  where
    S::Value: Lerp,
  {
    let Self { state, stops } = self;
    CustomLerpState::from_state(state, move |from, to, rate| {
      Self::sample_stops(&stops, from, to, rate)
    })
  }
}

/// Creates an animate state from a list of keyframes. This macro accepts the
/// following arguments:
///
/// * A state to use for the keyframes.
/// * A list of pairs of `rate` and `state_value` to specify the keyframes. The
///   `rate` is a float, where 0 represents the start, and 1 indicates the
///   finish, or a percentage suffixed by `%`.
///
/// # Examples
///
/// ```
/// use ribir::prelude::*;
///
/// let value = Stateful::new(100.0);
/// let frames_state = keyframes! {
///     state: value,
///     // accepts value
///     0.1 => 20.0,
///     0.5 => 60.0,
///     90% => 90.0
/// };
/// ```
///
/// Tuple states and tuple values are also accepted and are packed internally.
///
/// ```
/// use ribir::prelude::*;
///
/// let opacity = Stateful::new(0.0_f32);
/// let scale = Stateful::new(1.0_f32);
/// let frames_state = keyframes! {
///   state: (opacity, scale),
///   20% => (0.6, 1.05),
///   60% => (1.0, 0.95),
/// };
/// ```
///
/// Refer to the [module-level documentation](self) for more details on how to
/// use the macro in animations.
#[macro_export]
macro_rules! keyframes {
  (state: ($($state:expr),+ $(,)?), frames: [ $($f:expr),*] $(,)?) => {
    $crate::animation::KeyFrames::new(
        $crate::core::animate_state_pack!($($state),+),
        vec![ $($f),* ]
      )
      .into_lerp_fn_state()
    };
    (
      state: ($($state:expr),+ $(,)?),
      frames: [ $($f:expr),* ],
      $l: literal% => ($($v:expr),+ $(,)? )
      $(, $($rest:tt)*)?
    ) => {
      $crate::keyframes!(
        state: ($($state),+),
        frames: [
          $($f,)*
          $crate::animation::KeyFrame {
            rate: $l as f32 / 100.,
            state_value: $crate::core::animate_state_pack!($($v),+)
          }
        ]
        $(, $($rest)*)?
      )
    };
    (
      state: ($($state:expr),+ $(,)?),
      frames: [$($f:expr),*],
      $rate: expr => ($($v:expr),+ $(,)? )
      $(, $($rest:tt)*)?
    ) => {
      $crate::keyframes!(
        state: ($($state),+),
        frames: [
          $($f,)*
          $crate::animation::KeyFrame {
            rate: $rate,
            state_value: $crate::core::animate_state_pack!($($v),+)
          }
        ]
        $(, $($rest)*)?
      )
    };
    (state: ($($state:expr),+ $(,)?), $($rest: tt)+) => {
      $crate::keyframes!(state: ($($state),+), frames: [], $($rest)*)
    };
    (state: $state:expr, frames: [ $($f:expr),*] $(,)?) => {
      $crate::animation::KeyFrames::new($state, vec![ $($f),*]).into_lerp_fn_state()
    };
    (state: $state:expr, frames: [ $($f:expr),* ], $l: literal% => $v:expr $(, $($rest:tt)*)?) => {
      $crate::keyframes!(
        state: $state,
        frames: [
          $($f,)*
          $crate::animation::KeyFrame { rate: $l as f32 / 100., state_value: $v }
        ]
        $(, $($rest)*)?
      )
    };
    (state: $state:expr, frames: [$($f:expr),*], $rate: expr => $v:expr $(, $($rest:tt)*)?) => {
      $crate::keyframes!(
        state: $state,
        frames: [
          $($f,)*
          $crate::animation::KeyFrame { rate: $rate, state_value: $v }
        ]
        $(, $($rest)*)?
      )
    };
    (state: $state:expr, $($rest: tt)+) => {
      $crate::keyframes!(state: $state, frames: [], $($rest)*)
    };
  }
pub use keyframes;

#[cfg(test)]
mod tests {
  use ribir_core::{animate_state_pack, prelude::Stateful, reset_test_env};

  use super::*;

  #[test]
  fn smoke() {
    reset_test_env!();
    let state = Stateful::new(1.);
    let mut keyframes = keyframes! {
      state: state,
      0.1 => 0.4,
      50% => 2.5,
      1. => 1.
    };

    let p5 = keyframes.calc_lerp_value(&0., &1., 0.05);
    assert!(0. < p5 && p5 < 0.4);
    let p10 = keyframes.calc_lerp_value(&0., &1., 0.1);
    assert_eq!(p10, 0.4);
    let p25 = keyframes.calc_lerp_value(&0., &1., 0.25);
    assert!(0.4 < p25 && p25 < 2.5);
    let p50 = keyframes.calc_lerp_value(&0., &1., 0.5);
    assert_eq!(p50, 2.5);
    let p100 = keyframes.calc_lerp_value(&0., &1., 1.);
    assert_eq!(p100, 1.);
  }

  #[test]
  fn rate_zero_without_zero_stop_returns_from() {
    reset_test_env!();

    let frames = KeyFrames::new(
      Stateful::new(0.0_f32),
      vec![KeyFrame { rate: 0.25, state_value: 0.5 }, KeyFrame { rate: 0.75, state_value: 0.8 }],
    );

    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.2, &1.0, 0.0), 0.2);
    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.2, &1.0, -1.0), 0.2);
  }

  #[test]
  fn rate_zero_with_zero_stop_returns_stop_out_value() {
    reset_test_env!();

    let frames = KeyFrames::new(
      Stateful::new(0.0_f32),
      vec![
        KeyFrame { rate: 0.0, state_value: 0.3 },
        KeyFrame { rate: 0.0, state_value: 0.4 },
        KeyFrame { rate: 0.5, state_value: 0.8 },
      ],
    );

    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.1, &1.0, 0.0), 0.4);
  }

  #[test]
  fn duplicate_rate_preserves_step_semantics() {
    reset_test_env!();

    let frames = KeyFrames::new(
      Stateful::new(0.0_f32),
      vec![
        KeyFrame { rate: 0.5, state_value: 1.0 },
        KeyFrame { rate: 0.5, state_value: 0.25 },
        KeyFrame { rate: 1.0, state_value: 0.0 },
      ],
    );

    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 0.5), 0.25);
    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 0.25), 0.5);
    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 0.75), 0.125);
  }

  #[test]
  fn tail_segment_interpolates_to_to_without_hundred_percent_stop() {
    reset_test_env!();

    let frames =
      KeyFrames::new(Stateful::new(0.0_f32), vec![KeyFrame { rate: 0.5, state_value: 0.2 }]);

    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 0.75), 0.6);
  }

  #[test]
  fn rate_one_returns_to_even_with_hundred_percent_stop() {
    reset_test_env!();

    let frames =
      KeyFrames::new(Stateful::new(0.0_f32), vec![KeyFrame { rate: 1.0, state_value: 0.2 }]);

    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 1.0), 1.0);
    assert_eq!(KeyFrames::<Stateful<_>>::sample_stops(&frames.stops, &0.0, &1.0, 1.2), 1.0);
  }

  #[test]
  fn try_new_rejects_empty_frames() {
    reset_test_env!();

    let err = KeyFrames::try_new(Stateful::new(0.0_f32), vec![]).unwrap_err();
    assert_eq!(err, KeyFramesError::Empty);
  }

  #[test]
  fn try_new_rejects_non_finite_and_out_of_range_rates() {
    reset_test_env!();

    let nan_err = KeyFrames::try_new(
      Stateful::new(0.0_f32),
      vec![KeyFrame { rate: f32::NAN, state_value: 0.0 }],
    )
    .unwrap_err();
    assert!(matches!(nan_err, KeyFramesError::NonFiniteRate { index: 0, .. }));

    let inf_err = KeyFrames::try_new(
      Stateful::new(0.0_f32),
      vec![KeyFrame { rate: f32::INFINITY, state_value: 0.0 }],
    )
    .unwrap_err();
    assert!(matches!(inf_err, KeyFramesError::NonFiniteRate { index: 0, .. }));

    let low_err =
      KeyFrames::try_new(Stateful::new(0.0_f32), vec![KeyFrame { rate: -0.1, state_value: 0.0 }])
        .unwrap_err();
    assert_eq!(low_err, KeyFramesError::OutOfRangeRate { index: 0, rate: -0.1 });

    let high_err =
      KeyFrames::try_new(Stateful::new(0.0_f32), vec![KeyFrame { rate: 1.1, state_value: 0.0 }])
        .unwrap_err();
    assert_eq!(high_err, KeyFramesError::OutOfRangeRate { index: 0, rate: 1.1 });
  }

  #[test]
  fn new_panic_message_is_readable() {
    reset_test_env!();

    let panic = std::panic::catch_unwind(|| {
      let _ =
        KeyFrames::new(Stateful::new(0.0_f32), vec![KeyFrame { rate: -0.1, state_value: 0.0 }]);
    })
    .expect_err("invalid frames should panic");

    let message = panic_message(&panic);
    assert!(message.contains("Invalid KeyFrames"), "unexpected panic message: {message}");
    assert!(message.contains("out-of-range rate"), "unexpected panic message: {message}");
  }

  #[test]
  fn sample_supports_packed_values() {
    reset_test_env!();

    let frames = KeyFrames::new(
      Stateful::new(animate_state_pack!(0.0_f32, 0.0_f32)),
      vec![KeyFrame { rate: 0.5, state_value: animate_state_pack!(1.0, 0.25) }],
    );

    assert_eq!(
      KeyFrames::<Stateful<_>>::sample_stops(
        &frames.stops,
        &animate_state_pack!(0.0, 0.0),
        &animate_state_pack!(1.0, 1.0),
        0.25,
      ),
      animate_state_pack!(0.5, 0.125)
    );
  }

  #[test]
  fn macro_supports_tuple_state_and_values() {
    reset_test_env!();

    let opacity = Stateful::new(0.0_f32);
    let scale = Stateful::new(1.0_f32);

    let mut frames = crate::keyframes! {
      state: (opacity, scale),
      25% => (0.25, 1.25),
      50% => (1.0, 2.0),
    };

    assert_eq!(
      frames.calc_lerp_value(&animate_state_pack!(0.0, 1.0), &animate_state_pack!(1.0, 3.0), 0.125),
      animate_state_pack!(0.125, 1.125)
    );
    assert_eq!(
      frames.calc_lerp_value(&animate_state_pack!(0.0, 1.0), &animate_state_pack!(1.0, 3.0), 0.5),
      animate_state_pack!(1.0, 2.0)
    );
  }

  #[test]
  fn macro_tuple_state_preserves_duplicate_rate_semantics() {
    reset_test_env!();

    let opacity = Stateful::new(0.0_f32);
    let scale = Stateful::new(0.0_f32);

    let mut frames = crate::keyframes! {
      state: (opacity, scale),
      50% => (1.0, 2.0),
      50% => (0.0, 4.0),
      100% => (2.0, 0.0),
    };

    assert_eq!(
      frames.calc_lerp_value(&animate_state_pack!(0.0, 0.0), &animate_state_pack!(1.0, 1.0), 0.25),
      animate_state_pack!(0.5, 1.0)
    );
    assert_eq!(
      frames.calc_lerp_value(&animate_state_pack!(0.0, 0.0), &animate_state_pack!(1.0, 1.0), 0.5),
      animate_state_pack!(0.0, 4.0)
    );
    assert_eq!(
      frames.calc_lerp_value(&animate_state_pack!(0.0, 0.0), &animate_state_pack!(1.0, 1.0), 0.75),
      animate_state_pack!(1.0, 2.0)
    );
  }

  fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
      message.clone()
    } else if let Some(message) = panic.downcast_ref::<&'static str>() {
      (*message).to_string()
    } else {
      "<non-string panic payload>".to_string()
    }
  }

  mod macro_hygiene {
    use ribir_core::{animation::AnimateState, prelude::Stateful, reset_test_env};

    #[test]
    fn macro_does_not_require_keyframe_in_scope() {
      reset_test_env!();

      let mut frames = crate::keyframes! {
        state: Stateful::new(0.0_f32),
        50% => 1.0,
        100% => 0.0,
      };

      assert_eq!(frames.calc_lerp_value(&0.0, &1.0, 0.5), 1.0);
    }
  }
}
