//! A stagger animation consists of a sequence of animations that execute
//! consecutively. Each subsequent animation commences following a delay from
//! the start of the preceding animation, regardless of whether the preceding
//! animation has concluded.
//!
//! # Example
//!
//! You can add animations to a stagger animation in two ways:
//!
//! - add a animation to the stagger
//! - add states to the stagger with a "from" value.
//!
//! ```rust
//! use std::time::Duration;
//!
//! use ribir::prelude::*;
//!
//! let _ = fn_widget! {
//!   let mut stagger = Stagger::new(
//!     Duration::from_millis(100),
//!     transitions::EASE_IN.of(ctx!())
//!   );
//!
//!   let mut first = @Text { text: "first" };
//!   let mut second = @Text { text: "second" };
//!   let first_opacity = first
//!     .get_opacity_widget()
//!     .map_writer(|w| PartData::from_ref_mut(&mut w.opacity));
//!   let second_opacity = second
//!     .get_opacity_widget()
//!     .map_writer(|w| PartData::from_ref_mut(&mut w.opacity));
//!
//!
//!   let first_fade_in = @Animate {
//!     transition: transitions::EASE_IN.of(ctx!()),
//!      state: first_opacity,
//!   };
//!
//!   stagger.write().push_animation(first_fade_in);
//!   stagger.write().push_state(second_opacity, 0., ctx!());
//!
//!   @Column {
//!     on_mounted: move |_| stagger.run(),
//!     @{ [first, second] }
//!   }
//! };
//! ```

use ribir_algo::Sc;
use ribir_macros::rdl;

use super::*;
use crate::prelude::*;

/// The controller of a stagger animation. It's allow you to transition states
/// and run animation in a stagger way.
pub struct Stagger<T> {
  stagger: std::time::Duration,
  transition: Sc<T>,
  running_handle: Option<TaskHandle<NormalReturn<()>>>,
  next_to_run: Option<AnimationCursor>,
  animations: Vec<(std::time::Duration, Box<dyn Animation>)>,
  run_times: usize,
}

impl<T: Transition + 'static> Stagger<T> {
  /// **stagger**: the default duration between two adjacent animations start.
  /// **transition**: the transition for the states.
  pub fn new(stagger: Duration, transition: T) -> Stateful<Self> {
    Stateful::new(Self {
      stagger,
      transition: Sc::new(transition),
      running_handle: None,
      next_to_run: None,
      animations: vec![],
      run_times: 0,
    })
  }

  /// Add an new state as animation to the end of the stagger animation.
  ///
  /// **state**: the state you want to transition.
  /// **from**: the state you want to transition from.
  pub fn push_state<A>(&mut self, state: A, from: A::Value, ctx: &BuildCtx) -> State<Animate<A>>
  where
    A: AnimateState + 'static,
  {
    self.push_state_with(self.default_stagger(), state, from, ctx)
  }

  /// Add an new state as animation to the end of the stagger animation with a
  /// different stagger duration.
  ///
  /// - **stagger**： The duration between the previous animation start and the
  ///   `state` start transition.
  /// - **state**: the state you want to transition.
  /// - **from**: the state you want to transition from.
  pub fn push_state_with<A>(
    &mut self, stagger: Duration, state: A, from: A::Value, ctx!(): &BuildCtx,
  ) -> State<Animate<A>>
  where
    A: AnimateState + 'static,
  {
    let transition = Box::new(self.transition.clone());
    let animate = rdl! { Animate { transition, state, from } };
    self.push_animation_with(stagger, animate.clone_writer().into_inner());
    animate
  }

  /// Appends an animation to the back of a stagger animation.
  pub fn push_animation(&mut self, animation: impl Animation + 'static) -> &mut Self {
    self.push_animation_with(self.default_stagger(), animation)
  }

  /// Add an animation to the end of the stagger animation with a different
  /// stagger duration.
  ///
  /// **stagger**: the duration between the previous animation start and this
  /// animation start.
  pub fn push_animation_with(
    &mut self, stagger: Duration, animation: impl Animation + 'static,
  ) -> &mut Self {
    self
      .animations
      .push((stagger, Box::new(animation)));
    self
  }

  fn default_stagger(&self) -> Duration {
    if self.animations.is_empty() { Duration::ZERO } else { self.stagger }
  }
}

#[derive(Clone)]
struct AnimationCursor {
  prev_at: Instant,
  index: usize,
}

impl<T: Transition + 'static> Animation for Stateful<Stagger<T>> {
  fn run(&self) {
    if self.is_running() {
      self.stop()
    }
    {
      let mut this = self.write();
      this.next_to_run = Some(AnimationCursor { prev_at: Instant::now(), index: 0 });
      this.run_times += 1;
    }

    self.trigger_next();
  }

  fn is_running(&self) -> bool { self.read().is_running() }

  fn stop(&self) {
    if self.is_running() {
      let mut this = self.write();
      if let Some(h) = this.running_handle.take() {
        h.unsubscribe();
      }
      this.next_to_run.take();

      for (_, a) in this.animations.iter() {
        a.stop();
      }
    }
  }

  fn box_clone(&self) -> Box<dyn Animation> {
    let c = self.clone_writer().into_inner();
    Box::new(c)
  }
}

impl<T: Transition + 'static> Stateful<Stagger<T>> {
  fn trigger_next(&self) {
    let mut this = self.write();
    if let Some(step) = this.next_to_run.take() {
      if let Some((delay, next)) = this.animations.get(step.index) {
        let at = step.prev_at + *delay;
        let next = next.box_clone();
        this.next_to_run = Some(AnimationCursor { prev_at: at, index: step.index + 1 });

        // the status not changed(running -> running), so we can ignore the
        // modification.
        this.forget_modifies();
        drop(this);

        let this = self.clone_writer().into_inner();
        let h = observable::timer_at((), at, AppCtx::scheduler()).subscribe(move |_| {
          next.run();
          this.trigger_next();
        });
        self.write().running_handle = Some(h);
      } else {
        this.running_handle = None;
      }
    }
  }
}

impl<T> Stagger<T> {
  /// Check if the stagger animation is running.
  pub fn is_running(&self) -> bool {
    self.running_handle.is_some()
      || self.next_to_run.is_some()
      || self
        .animations
        .iter()
        .any(|(_, a)| a.is_running())
  }

  /// How many times the stagger animation has run.
  pub fn run_times(&self) -> usize { self.run_times }

  /// Check if the stagger animation has ever run.
  pub fn has_ever_run(&self) -> bool { self.run_times > 0 }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  fn stagger_run_and_stop() -> impl WidgetBuilder {
    fn_widget! {
      let stagger = Stagger::new(Duration::from_millis(100), transitions::EASE_IN.of(ctx!()));
      let mut mock_box = @MockBox { size: Size::new(100., 100.) };
      let opacity = mock_box
        .get_opacity_widget()
        .map_writer(|w| PartData::from_ref_mut(&mut w.opacity));
      let animate = @Animate {
        transition: transitions::EASE_IN.of(ctx!()),
        state: opacity,
        from: 0.,
      };

      stagger.write().push_animation(animate);
      stagger.write().push_state(
        mock_box.map_writer(|w| PartData::from_ref_mut(&mut w.size)),
        Size::new(200., 200.),
        ctx!()
      );


      stagger.run();
      assert!(stagger.is_running());
      stagger.stop();
      assert!(!stagger.is_running());

      mock_box
    }
  }
  widget_layout_test!(stagger_run_and_stop, width == 100., height == 100.,);

  #[test]
  fn stagger_not_running_after_all_animation_end() {
    reset_test_env!();

    let stagger = Stagger::new(
      Duration::from_millis(100),
      EasingTransition { duration: Duration::ZERO, easing: easing::LINEAR },
    );
    let c_stagger = stagger.clone_writer().into_inner();
    let w = fn_widget! {
      let mut mock_box = @MockBox { size: Size::new(100., 100.) };
      $stagger.write().push_state(
        mock_box
          .get_opacity_widget()
          .map_writer(|w| PartData::from_ref_mut(&mut w.opacity)),
        0.,
        ctx!()
      );
      stagger.run();

      mock_box
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    // draw twice to ensure the 'zero' animation is finished.
    wnd.draw_frame();
    assert!(!c_stagger.is_running());
  }
}
