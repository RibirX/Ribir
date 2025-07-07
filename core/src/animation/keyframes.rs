//! Keyframes are used to manage the intermediate steps of animation states.
//!
//! This offers a straightforward method to define the state value and ensure
//! seamless animation transitions between the keyframes.
//!
//! You can employ the `keyframes!` macro to generate an animated state.
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
//!   let mut color_rect = @SizedBox {
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
//!   let mut opacity_rect = @SizedBox { size: Size::new(100., 100.) };
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
//!     }.box_it()
//!   };
//!
//!   @(opacity_rect) {
//!     on_tap: move |_| animate.run()
//!   }
//! };
//! ```
#[derive(Debug)]
pub struct KeyFrames<S: AnimateStateSetter> {
  /// The state for the keyframes.
  pub state: S,
  /// The keyframes that specify the animation frames.
  frames: Box<[KeyFrame<S::Value>]>,
}

#[derive(Debug)]
pub struct KeyFrame<S> {
  pub rate: f32,
  pub state_value: S,
}

impl<S: AnimateStateSetter> KeyFrames<S> {
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
  /// Panics if the `keyframes` vector is empty.
  ///
  /// # Remarks
  ///
  /// The keyframes are sorted by their rate in ascending order before being
  /// stored.
  pub fn new(state: S, mut keyframes: Vec<KeyFrame<S::Value>>) -> Self {
    assert!(!keyframes.is_empty(), "KeyFrames must have at least one frame");
    keyframes.sort_by(|a, b| a.rate.total_cmp(&b.rate));

    Self { state, frames: keyframes.into_boxed_slice() }
  }

  #[allow(clippy::type_complexity)]
  /// Converts the `KeyFrames` into a `LerpFnState` that can be used for
  /// animations.
  pub fn into_lerp_fn_state(
    self,
  ) -> LerpFnState<S, impl FnMut(&S::Value, &S::Value, f32) -> S::Value>
  where
    S::Value: Lerp,
  {
    let Self { state, frames } = self;
    LerpFnState::new(state, move |from, to, rate| {
      let idx = frames
        .binary_search_by(|f| f.rate.total_cmp(&rate))
        .unwrap_or_else(|idx| idx);

      if idx == 0 {
        let rate = if rate > 0. { rate / frames[0].rate } else { 1. };
        from.lerp(&frames[0].state_value, rate)
      } else if idx == frames.len() {
        let pre_rate = frames[idx - 1].rate;
        if pre_rate == 1. {
          to.clone()
        } else {
          let rate = (rate - pre_rate) / (1. - pre_rate);
          frames[idx - 1].state_value.lerp(to, rate)
        }
      } else {
        let f2 = &frames[idx];
        let f1 = &frames[idx - 1];
        if f2.rate == f1.rate {
          f2.state_value.clone()
        } else {
          let rate = (rate - f1.rate) / (f2.rate - f1.rate);
          f1.state_value.lerp(&f2.state_value, rate)
        }
      }
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
/// let value = State::value(100.0);
/// let frames_state = keyframes! {
///     state: value,
///     // accepts value
///     0.1 => 20.0,
///     0.5 => 60.0,
///     90% => 90.0
/// };
/// ```
///
/// Refer to the [module-level documentation](self) for more details on how to
/// use the macro in animations.
#[macro_export]
macro_rules! keyframes {
  (state: $state:expr, frames: [ $($f:expr),*] $(,)?) => {
    $crate::animation::KeyFrames::new($state, vec![ $($f),*]).into_lerp_fn_state()
  };
  (state: $state:expr, frames: [ $($f:expr),* ], $l: literal% => $v:expr $(, $($rest:tt)*)?) => {
    $crate::keyframes!(
      state: $state,
      frames: [
        $($f,)*
        KeyFrame { rate: $l as f32 / 100., state_value: $v }
      ]
      $(, $($rest)*)?
    )
  };
  (state: $state:expr, frames: [$($f:expr),*], $rate: expr => $v:expr $(, $($rest:tt)*)?) => {
    $crate::keyframes!(
      state: $state,
      frames: [
        $($f,)*
        KeyFrame { rate: $rate, state_value: $v }
      ]
      $(, $($rest)*)?
    )
  };
  (state: $state:expr, $($rest: tt)+) => {
    $crate::keyframes!(state: $state, frames: [], $($rest)*)
  };
}
pub use keyframes;

use super::{AnimateStateSetter, Lerp, LerpFnState};
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{animation::animate_state::AnimateState, state::State};

  #[test]
  fn smoke() {
    let state = State::value(1.);
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
}
