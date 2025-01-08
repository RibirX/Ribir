use easing::CubicBezierEasing;
use ribir_core::{prelude::*, smooth_width};
use ribir_widgets::prelude::*;

use crate::md;

const DETERMINATE_TRANS: EasingTransition<CubicBezierEasing> = EasingTransition {
  easing: md::easing::EMPHASIZED_DECELERATE,
  duration: md::easing::duration::LONG4,
};

fn indeterminate_trans() -> Box<dyn Transition> {
  EasingTransition { easing: md::easing::EMPHASIZED, duration: Duration::from_secs(2) }
    .repeat(f32::INFINITY)
    .box_it()
}

class_names! {
  MD_BASE_LINEAR_INDICATOR,
  MD_BASE_SPINNER,
  MD_BASE_SPINNER_INDICATOR,
  MD_BASE_SPINNER_TRACK
}

fn lerp_angle(from: &Angle, to: &Angle, rate: f32) -> Angle {
  let radians = from.radians.lerp(&to.radians, rate);
  Angle::radians(radians)
}
pub(super) fn init(classes: &mut Classes) {
  classes.insert(MD_BASE_LINEAR_INDICATOR, style_class! {
    background: BuildCtx::color(),
    border_radius: md::RADIUS_2,
  });
  classes.insert(MD_BASE_SPINNER, style_class! {
    clamp: BoxClamp::fixed_size(md::SIZE_48),
    painting_style: PaintingStyle::Stroke(StrokeOptions {
      width: md::THICKNESS_4,
      line_cap: LineCap::Round,
      ..Default::default()
    }),
  });
  classes.insert(MD_BASE_SPINNER_INDICATOR, style_class! {
    class: MD_BASE_SPINNER,
    foreground: BuildCtx::color(),
  });
  classes.insert(MD_BASE_SPINNER_TRACK, style_class! {
    class: MD_BASE_SPINNER,
    foreground: BuildCtx::container_color(),
  });
  classes.insert(LINEAR_INDETERMINATE_TRACK, style_class! {
    background: BuildCtx::container_color(),
    border_radius: md::RADIUS_2,
    margin: md::EDGES_LEFT_4,
  });
  classes.insert(LINEAR_PROGRESS, style_class! {
    margin: md::EDGES_HOR_4,
    clamp: BoxClamp::EXPAND_X.with_fixed_height(md::THICKNESS_4)
  });
  classes.insert(LINEAR_DETERMINATE_TRACK, |w| {
    let w = FatObj::new(w);
    stack! {
      @ $w { class: LINEAR_INDETERMINATE_TRACK }
      // As part of the Material Design 3 theme, a stop indicator has been
      // introduced for the progress.
      @Container {
        h_align: HAlign::Right,
        class: MD_BASE_LINEAR_INDICATOR,
        size: Size::new(md::THICKNESS_4, md::THICKNESS_4),
      }
    }
    .into_widget()
  });
  classes.insert(LINEAR_DETERMINATE_INDICATOR, move |host| {
    let host = FatObj::new(host);
    smooth_width! {
      transition: DETERMINATE_TRANS,
      init_value: 0.,
      @ $host { class: MD_BASE_LINEAR_INDICATOR }
    }
    .into_widget()
  });
  classes.insert(LINEAR_INDETERMINATE_INDICATOR, move |host| {
    // We expanded the indicator to a `Row[Indicator, Track, Indicator]` structure,
    // thus transforming the entire progress into `Row[Indicator, Track,
    // Indicator, Track]`, adjusting the size of the four children to simulate
    // the repeated motion.
    let host = FatObj::new(host);
    fn_widget! {
      let indicator1 = @Expanded { flex: 0.};
      let track1 = @Expanded { flex: 0.,  };
      let indicator2 = @Expanded { flex: 0.};
      let total_fraction = @FractionallyWidthBox { factor: 0. };

      @Animate {
        transition: indeterminate_trans(),
        state: keyframes! {
          state: (
            part_writer!(&mut indicator1.flex),
            part_writer!(&mut track1.flex),
            part_writer!(&mut indicator2.flex),
            part_writer!(&mut total_fraction.factor),
          ),
          15% => (0., 0., 0.15, 0.15),
          45% => (0., 0.5, 0.5, 1.),
          55% => (0.15, 0.6, 0.25, 1.),
          60% => (0.35, 0.65, 0., 1.),
          60% => (0., 0., 0.35, 0.35),
          80% => (0., 0.4, 0.6, 1.),
          95% => (0., 1., 0., 1.),
          95% => (0., 0., 0., 0.),
        },
        from: (0., 0., 0., 0.)
      }.run();

      @ $total_fraction {
        @Row {
          @ $indicator1 { @ $host { class: MD_BASE_LINEAR_INDICATOR } }
          @ $track1 {
            @FractionallyWidthBox { class: LINEAR_INDETERMINATE_TRACK }
          }
          @ $indicator2 {
            @FractionallyWidthBox {
              margin: EdgeInsets::only_left(4.),
              class: MD_BASE_LINEAR_INDICATOR
            }
          }
        }
      }
    }
    .into_widget()
  });

  classes.insert(SPINNER_DETERMINATE, move |w| {
    let w = FatObj::new(w);
    let margin_angle: Angle = Angle::degrees(16.);
    fn_widget! {
      let indicator = Provider::of::<Stateful<SpinnerArc>>(BuildCtx::get()).unwrap();
      let track = @SpinnerArc {
        start: distinct_pipe! {
          let indicator = $indicator;
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
        @ $track { class: MD_BASE_SPINNER_TRACK }
        @ $w { class: MD_BASE_SPINNER_INDICATOR }
      }

    }
    .into_widget()
  });

  classes.insert(SPINNER_INDETERMINATE, move |w| {
    let w = FatObj::new(w);
    fn_widget! {
      let indicator = Provider::of::<Stateful<SpinnerArc>>(BuildCtx::get()).unwrap();
      let pi = Angle::pi();
      @Animate {
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
      }.run();

      @ $w { class: MD_BASE_SPINNER_INDICATOR }
    }
    .into_widget()
  });
}
