use easing::CubicBezierEasing;
use ribir_core::{prelude::*, smooth_width};
use ribir_widgets::prelude::*;

use crate::md;

const DETERMINATE_TRANS: EasingTransition<CubicBezierEasing> = EasingTransition {
  easing: md::easing::EMPHASIZED_DECELERATE,
  duration: md::easing::duration::LONG4,
};

fn indeterminate_trans() -> impl Transition {
  EasingTransition { easing: md::easing::EMPHASIZED, duration: Duration::from_secs(2) }
    .repeat(f32::INFINITY)
}

fn md_base_spinner(w: Widget, foreground: PipeValue<Brush>) -> Widget {
  fat_obj! {
    foreground,
    clamp: BoxClamp::fixed_size(md::SIZE_48),
    painting_style: PaintingStyle::Stroke(StrokeOptions {
      width: md::THICKNESS_4,
      line_cap: LineCap::Round,
      ..Default::default()
    }),
    @ { w }
  }
  .into_widget()
}

named_style_impl! { md_base_linear_indicator => {
  background: BuildCtx::color(),
  radius: md::RADIUS_2,
}}

fn lerp_angle(from: &Angle, to: &Angle, rate: f32) -> Angle {
  let radians = from.radians.lerp(&to.radians, rate);
  Angle::radians(radians)
}
pub(super) fn init(classes: &mut Classes) {
  named_style_impl! { linear_base_track => {
    background: BuildCtx::container_color(),
    radius: md::RADIUS_2,
    margin: md::EDGES_LEFT_4,
  }};

  classes.insert(
    LINEAR_PROGRESS,
    style_class! {
      margin: md::EDGES_HOR_4,
      clamp: BoxClamp::UNLIMITED.with_fixed_height(md::THICKNESS_4)
    },
  );
  classes.insert(LINEAR_DETERMINATE_TRACK, |w| {
    stack! {
      fit: StackFit::Passthrough,
      @linear_base_track(w)
      // As part of the Material Design 3 theme, a stop indicator has been
      // introduced for the progress.
      @InParentLayout {
        @Container {
          x: AnchorX::right(),
          background: BuildCtx::color(),
          radius: md::RADIUS_2,
          size: Size::new(md::THICKNESS_4, md::THICKNESS_4),
        }
      }
    }
    .into_widget()
  });
  classes.insert(LINEAR_DETERMINATE_INDICATOR, move |host| {
    smooth_width! {
      transition: DETERMINATE_TRANS,
      init_value: 0.,
      @md_base_linear_indicator(host)
    }
    .into_widget()
  });
  classes.insert(LINEAR_PROGRESS_INDETERMINATE, move |host| {
    // We expanded the indicator to a `Row[Indicator, Track, Indicator]` structure,
    // thus transforming the entire progress into `Row[Indicator, Track,
    // Indicator, Track]`, adjusting the size of the four children to simulate
    // the repeated motion.
    fn_widget! {
      let indicator1 = @Expanded { flex: 0.};
      let track1 = @Expanded { flex: 0.,  };
      let indicator2 = @Expanded { flex: 0.};
      let track2 = @Expanded { flex: 1. };

      @Animate {
        transition: indeterminate_trans(),
        state: keyframes! {
          state: (
            part_writer!(&mut indicator1.flex),
            part_writer!(&mut track1.flex),
            part_writer!(&mut indicator2.flex),
            part_writer!(&mut track2.flex),
          ),
          15% => (0., 0., 0.15, 0.85),
          45% => (0., 0.5, 0.5, 0.),
          55% => (0.15, 0.6, 0.25, 0.),
          60% => (0.35, 0.65, 0., 0.),
          60% => (0., 0., 0.35, 0.65),
          80% => (0., 0.4, 0.6, 0.),
          95% => (0., 1., 0., 0.),
          95% => (0., 0., 0., 1.),
        },
        from: (0., 0., 0., 1.)
      }.run();

      @Flex {
        class: LINEAR_PROGRESS,
        align_items: Align::Stretch,
        @(indicator1) { @md_base_linear_indicator(host) }
        @(track1) { @linear_base_track(Void::default().into_widget()) }
        @(indicator2) {
          @Margin {
            margin: EdgeInsets::only_left(4.),
            @md_base_linear_indicator(Void::default().into_widget())
          }
        }
        @(track2) { @linear_base_track(Void::default().into_widget()) }
      }
    }
    .into_widget()
  });

  classes.insert(SPINNER_DETERMINATE, move |w| {
    let margin_angle: Angle = Angle::degrees(16.);
    fn_widget! {
      let indicator = Provider::of::<Stateful<SpinnerArc>>(BuildCtx::get()).unwrap();
      let track = @SpinnerArc {
        start: distinct_pipe! {
          let indicator = $read(indicator);
          if indicator.offset_angle().to_degrees().abs() < 0.1 {
            Angle::zero()
          } else {
            indicator.end + margin_angle
          }
        },
        end: Angle::two_pi() - margin_angle,
      };

      // We use a custom lerp function to calculate the angle without
      // considering if there is a short arc to traverse.
      LerpFnState::new(part_writer!(&mut indicator.end), lerp_angle)
        .transition(DETERMINATE_TRANS);
      LerpFnState::new(part_writer!(&mut track.start), lerp_angle)
        .transition(DETERMINATE_TRANS);
      let center = md::SIZE_48 / 2.;
      @Stack {
        transform: Transform::translation(-center.width, -center.height)
          .then_rotate(margin_angle / 2.)
          .then_translate(center.to_vector()),
        @md_base_spinner(track.into_widget(), BuildCtx::container_color().r_into())
        @md_base_spinner(w, BuildCtx::color().r_into())
      }

    }
    .into_widget()
  });

  classes.insert(SPINNER_INDETERMINATE, move |w| {
    fn_widget! {
      let indicator = Provider::of::<Stateful<SpinnerArc>>(BuildCtx::get()).unwrap();
      let pi = Angle::pi();
      let infinite_animate = @Animate {
        state: keyframes!{
          state: (part_writer!(&mut indicator.start), part_writer!(&mut indicator.end)),
          20% => (pi * 0.5, pi * 1.),
          35% => (pi * 0.75, pi * 1.99),
          50% => (pi * 1.25, pi * 2.75),
          50% => (pi * -0.75, pi * 0.75),
          65% => (pi * 0.24, pi * 1.),
          80% => (pi * 1.23, pi * 1.5),
          100% => (pi * 2., pi * 2.1),
          100% => (pi * 0., pi * 0.1),
        },
        transition: indeterminate_trans(),
        from: (Angle::zero(), pi * 0.1),
      };
      infinite_animate.run();
      @FatObj {
        on_disposed: move |_| infinite_animate.stop(),
        @md_base_spinner(w, BuildCtx::color().r_into())
      }
    }
    .into_widget()
  });
}
