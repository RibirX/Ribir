use ribir_core::prelude::*;

use crate::layout::{Stack, StackFit};

class_names! {
  #[doc = "Class name for the indicator of the linear progress"]
  LINEAR_PROGRESS_INDICATOR,
  #[doc = "Class name for the track of the linear progress"]
  LINEAR_PROGRESS_TRACK,
  #[doc = "Class name for the linear determinate progress"]
  LINEAR_DETERMINATE_PROGRESS,
  #[doc = "Class name for the linear indeterminate progress"]
  LINEAR_INDETERMINATE_PROGRESS,
  #[doc = "Class name for the indicator of the circle progress"]
  CIRCLE_PROGRESS_INDICATOR,
  #[doc = "Class name for the track of the circle progress"]
  CIRCLE_PROGRESS_TRACK,
  #[doc = "Class name for the circle determinate progress"]
  CIRCLE_DETERMINATE_PROGRESS,
  #[doc = "Class name for the circle indeterminate progress"]
  CIRCLE_INDETERMINATE_PROGRESS
}

/// the widget that shows progress along a line.
#[derive(Declare)]
pub struct LinearProgress {
  /// there are two kind of linear progress.
  /// 1.Determinate, when the value is Some(xx).
  ///   Determinate progress indicators have a specific value at each point in
  ///   time. And the value is between 0.0 and 1.0, meaning how much progress
  ///   has passed. 0.0 means no progress and 1.0 means that progress is
  ///   complete.
  /// 2.Indeterminate, when the value is None.
  ///   Indeterminate means that progress is being made without indicating how
  ///   much progress has passed and remains.
  pub value: Option<f32>,
}

/// the widget that shows progress along a circle.
#[derive(Declare)]
pub struct CircleProgress {
  /// there are two kind of linear progress.
  /// 1.Determinate, when the value is Some(xx).
  ///   Determinate progress indicators have a specific value at each point in
  ///   time. And the value is between 0.0 and 1.0, meaning how much progress
  ///   has passed. 0.0 means no progress and 1.0 means that progress is
  ///   complete.
  /// 2.Indeterminate, when the value is None.
  ///   Indeterminate means that progress is being made without indicating how
  ///   much progress has passed and remains.
  pub value: Option<f32>,
}

impl Compose for LinearProgress {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      pipe!($this.value.is_some())
        .value_chain(|s| s.distinct_until_changed().box_it())
        .map(move |is_determinate| {
        if is_determinate {
          let mut track = @Stack {
            class: LINEAR_PROGRESS_TRACK,
            clamp: BoxClamp::EXPAND_X,
          };
          let indicator = @Container {
            size: distinct_pipe! {
              let width = $this.value.unwrap() * $track.layout_width();
              Size::new(width, f32::MAX)
            },
          };
          let provider = Provider::new(Box::new(indicator.clone_writer()));
          @ provider.with_child(fn_widget! {
            @$track {
              class: LINEAR_DETERMINATE_PROGRESS,
              @$indicator { class: LINEAR_PROGRESS_INDICATOR }
            }
          })
          .into_widget()
        } else {
          @Void {
            class: LINEAR_INDETERMINATE_PROGRESS,
          }
          .into_widget()
        }
      })
    }
    .into_widget()
  }
}

impl Compose for CircleProgress {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
        pipe!($this.value.is_some())
          .value_chain(|s| s.distinct_until_changed().box_it())
          .map(move |is_determinate| {
            if is_determinate {
              let mut stack = @Stack { fit: StackFit::Expand };
              let indicator = @ArcStroke {
                center: distinct_pipe! {
                  Point::new($stack.layout_size().width / 2., $stack.layout_size().height / 2.)
                },
                radius: distinct_pipe! { $stack.layout_size().width / 2. },
                start_angle: Angle::zero(),
                offset_angle: distinct_pipe! {  Angle::two_pi() * $this.value.unwrap() },
              };
              let track = @ArcStroke {
                center: distinct_pipe! {
                  Point::new($stack.layout_size().width / 2., $stack.layout_size().height / 2.)
                },
                radius: distinct_pipe! { $stack.layout_size().width / 2. },
                start_angle: Angle::zero(),
                offset_angle: Angle::two_pi(),
              };

              let provider = Provider::new(Box::new(indicator.clone_writer()));
              @ provider.with_child(fn_widget!{
                @$stack {
                  class: CIRCLE_DETERMINATE_PROGRESS,
                  @$ track { class: CIRCLE_PROGRESS_TRACK }
                  @$ indicator { class: CIRCLE_PROGRESS_INDICATOR }
                }
              }).into_widget()
            } else {
              @Void {
                class: CIRCLE_INDETERMINATE_PROGRESS,
              }
              .into_widget()
            }
          }
        )
    }
    .into_widget()
  }
}

/// the angle is travels in the direction given by clockwise.
/// and the position of 0 degrees corresponds to position of 12 o'clock
#[derive(Declare)]
pub struct ArcStroke {
  pub center: Point,
  pub radius: f32,
  pub start_angle: Angle,
  pub offset_angle: Angle,
}

impl Render for ArcStroke {
  fn only_sized_by_parent(&self) -> bool { true }

  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    Size::new(self.radius, self.radius)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let radius = self.radius.min(size.width).min(size.height);
    let start_angle: Angle<f32> = self.start_angle - Angle::degrees(90.);
    let end_angle = start_angle + self.offset_angle;
    if self.offset_angle.to_degrees() < 0.1 {
      return;
    }

    let x = radius * (1. + start_angle.radians.cos());
    let y = radius * (1. + start_angle.radians.sin());
    let painter = ctx.painter();
    if self.start_angle != self.offset_angle {
      painter.begin_path(Point::new(x, y));
      painter.arc_to(Point::new(radius, radius), radius, start_angle, end_angle);
      painter.end_path(false);
      painter.stroke();
    }
  }
}
