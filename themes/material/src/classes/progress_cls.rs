use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

const LINEAR_RADIUS: Radius = Radius::all(2.);
const LINEAR_INDETERMINATE_DURATION: Duration = Duration::from_millis(2000);
const LINEAR_MARGIN: f32 = 4.;
const LINEAR_INDETERMINATE_MAX_LEN: f32 = 0.8;
const LINEAR_THICKNESS: f32 = 4.;

const CIRCLE_INDETERMINATE_DURATION: Duration = Duration::from_millis(2000);
const CIRCLE_INDETERMINATE_MAX_DEGREE: f32 = 260.;
const CIRCLE_INDETERMINATE_MIN_DEGREE: f32 = 20.;
const CIRCLE_RADIUS: f32 = 24.;
const CIRCLE_SIZE: Size = Size::new(CIRCLE_RADIUS * 2., CIRCLE_RADIUS * 2.);
const CIRCLE_MARGIN: f32 = 4.;
const CIRCLE_PADDING: f32 = 2.;
const CIRCLE_THICKNESS: f32 = 4.;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(LINEAR_PROGRESS_TRACK, |w| {
    fn_widget! {
        @$w {
          background: Palette::of(ctx!()).secondary_container(),
          border_radius: LINEAR_RADIUS,
          clamp: BoxClamp::min_width(0.).with_fixed_height(LINEAR_THICKNESS)
        }
    }
    .into_widget()
  });

  classes.insert(LINEAR_PROGRESS_INDICATOR, |w| {
    fn_widget! {
      @$ w{
        background: Palette::of(ctx!()).primary(),
        border_radius: LINEAR_RADIUS,
      }
    }
    .into_widget()
  });

  classes.insert(LINEAR_DETERMINATE_PROGRESS, |w| {
    fn_widget! {
      let indicator = Provider::of::<Stateful<Container>>(ctx!()).unwrap().clone_writer();
      indicator.map_writer(|w| PartData::from_ref_mut(&mut w.size.width))
      .transition(transitions::LINEAR.of(ctx!()), ctx!());
      @$ w {
        margin: EdgeInsets::horizontal(LINEAR_MARGIN),
      }
    }
    .into_widget()
  });

  classes.insert(LINEAR_INDETERMINATE_PROGRESS, move |_| {
    const MAX_ROUND: f32 = 2.;
    fn total_len(rate: f32) -> f32 {
      if rate > 1. - 1. / MAX_ROUND {
        (rate * LINEAR_INDETERMINATE_MAX_LEN).min(1. - position(rate))
      } else {
        rate * LINEAR_INDETERMINATE_MAX_LEN
      }
    }
    fn position(rate: f32) -> f32 { (rate * MAX_ROUND) % 1. }

    fn_widget! {
      let mut track = @Stack {
        class: LINEAR_PROGRESS_TRACK,
        clamp: BoxClamp::EXPAND_X,
      };
      let mut indicator1 = @Container {
        size: Size::new(0., f32::MAX)
      };
      let indicator2 = @Container {
        size: Size::new(0., f32::MAX)
      };
      let transition = EasingTransition {
        easing: easing::LINEAR,
        duration: LINEAR_INDETERMINATE_DURATION,
      }.repeat(f32::MAX);

      let _trigger = Stateful::new(0.);
      @Animate {
        transition: transition.box_clone(),
        state: LerpFnState::new(
          _trigger,
          move |_, _, rate| {
            let len = total_len(rate);
            let pos = position(rate);
            $indicator1.write().size.width = len.min(1. - pos) * $track.layout_width();
            $indicator1.write().anchor = Anchor::left(pos * $track.layout_width());
            $indicator2.write().size.width = (pos + len - 1.).max(0.) * $track.layout_width();
            rate
          }
        ),
        from: 0.,
      }.run();

      @$track {
        clamp: BoxClamp::EXPAND_X,
        margin: EdgeInsets::horizontal(LINEAR_MARGIN),
        v_align: VAlign::Center,
        h_align: HAlign::Center,
        @$ indicator1 { class: LINEAR_PROGRESS_INDICATOR,}
        @$ indicator2 { class: LINEAR_PROGRESS_INDICATOR }
      }
    }
    .into_widget()
  });

  classes.insert(CIRCLE_PROGRESS_TRACK, move |w| {
    fn_widget! {
      @Container {
        size: CIRCLE_SIZE,
        padding: EdgeInsets::all(CIRCLE_PADDING),
        foreground: Palette::of(ctx!()).secondary_container(),
        painting_style: PaintingStyle::Stroke(StrokeOptions {
          width: CIRCLE_THICKNESS,
          miter_limit: 0.0,
          line_cap: LineCap::Round,
          line_join: LineJoin::default(),
        }),

        @ $w {}
      }
    }
    .into_widget()
  });

  classes.insert(CIRCLE_PROGRESS_INDICATOR, move |w| {
    fn_widget! {
      @Container {
        size: CIRCLE_SIZE,
        padding: EdgeInsets::all(CIRCLE_PADDING),
        foreground: Palette::of(ctx!()).primary(),
        painting_style: PaintingStyle::Stroke(StrokeOptions {
          width: CIRCLE_THICKNESS,
          miter_limit: 0.0,
          line_cap: LineCap::Round,
          line_join: LineJoin::default(),
        }),
        @ $w {}
      }
    }
    .into_widget()
  });

  classes.insert(CIRCLE_DETERMINATE_PROGRESS, move |w| {
    fn_widget! {
      let indicator = Provider::of::<Stateful<ArcStroke>>(ctx!()).unwrap().clone_writer();
      indicator.map_writer(|w| PartData::from_ref_mut(&mut w.offset_angle))
          .transition(transitions::LINEAR.of(ctx!()), ctx!());
      @Container {
        v_align: VAlign::Center,
        h_align: HAlign::Center,
        margin: EdgeInsets::all(CIRCLE_MARGIN),
        size: CIRCLE_SIZE,
        @$w {}
      }
    }
    .into_widget()
  });

  classes.insert(CIRCLE_INDETERMINATE_PROGRESS, move |_| {
    fn_widget! {
      let indicator = @ArcStroke {
        center: Point::new(CIRCLE_RADIUS, CIRCLE_RADIUS),
        radius: CIRCLE_RADIUS,
        start_angle: Angle::zero(),
        offset_angle: Angle::zero(),
      };

      let transition = EasingTransition {
        easing: easing::EASE_IN_OUT,
        duration: CIRCLE_INDETERMINATE_DURATION,
      }.repeat(f32::MAX);

      let base = Angle::two_pi() * 3. - Angle::degrees(CIRCLE_INDETERMINATE_MAX_DEGREE);
      let _trigger = Stateful::new(0.);
      @Animate {
        transition: transition.box_clone(),
        state: LerpFnState::new(
          _trigger,
          move |_, _, rate| {
            let min_gap = Angle::degrees(CIRCLE_INDETERMINATE_MIN_DEGREE);
            let max_gap = Angle::degrees(CIRCLE_INDETERMINATE_MAX_DEGREE);
            let change_gap = max_gap - min_gap;
            let mut indicator = $indicator.write();
            if rate < 0.5 {
              indicator.start_angle =  base * rate;
              indicator.offset_angle = change_gap * easing::EASE_IN.easing(2. * rate) + min_gap;
            } else {
              indicator.offset_angle = change_gap * easing::EASE_OUT.easing(2. * rate) + min_gap;
              indicator.start_angle =  base * rate + max_gap - indicator.offset_angle + min_gap;
            }
            rate
          }
        ),
        from: 0.,
      }.run();

      @Stack {
        clamp: BoxClamp::fixed_size(CIRCLE_SIZE),
        v_align: VAlign::Center,
        h_align: HAlign::Center,
        @ArcStroke {
          class: CIRCLE_PROGRESS_TRACK,
          center: Point::new(CIRCLE_RADIUS, CIRCLE_RADIUS),
          radius: CIRCLE_RADIUS,
          start_angle: Angle::zero(),
          offset_angle: Angle::two_pi(),
        }
        @$ indicator {
          class: CIRCLE_PROGRESS_INDICATOR,
        }
      }
    }
    .into_widget()
  });
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  fn progress_widget(_: &mut BuildCtx) -> Widget<'static> {
    fn_widget! {
      @Column {
        @Container {
          size: Size::new(128., 30.),
          @LinearProgress {
            v_align: VAlign::Center,
            value: Some(0.2)
          }
        }
        @Container {
          size: Size::new(128., 30.),
          @CircleProgress { value: Some(0.2) }
        }
      }
    }
    .into_widget()
  }

  widget_image_tests!(
    progress_widget,
    WidgetTester::new(progress_widget)
      .with_wnd_size(Size::new(200., 100.))
      .with_comparison(0.002)
  );
}
