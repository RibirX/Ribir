use ribir_core::prelude::*;

use crate::layout::*;

class_names! {
  /// Class name for the indeterminate linear progress
  LINEAR_PROGRESS_INDETERMINATE,
  #[doc = "Class name for the whole linear progress"]
  LINEAR_PROGRESS,
  #[doc = "Class name for the track of the determinate linear progress"]
  LINEAR_DETERMINATE_TRACK,
  #[doc = "Class name for the linear progress determinate indicator"]
  LINEAR_DETERMINATE_INDICATOR,
  #[doc = "Class name for the determine spinner progress"]
  SPINNER_DETERMINATE,
  #[doc = "Class name for the indeterminate spinner progress"]
  SPINNER_INDETERMINATE,
}

/// The widget that shows progress along a line.
#[derive(Declare, Clone)]
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
  #[declare(default)]
  pub value: Option<f32>,
}

/// The widget displays progress along a spinner.
#[derive(Declare, Clone)]
pub struct SpinnerProgress {
  /// there are two kind of linear progress.
  /// 1.Determinate, when the value is Some(xx).
  ///   Determinate progress indicators have a specific value at each point in
  ///   time. And the value is between 0.0 and 1.0, meaning how much progress
  ///   has passed. 0.0 means no progress and 1.0 means that progress is
  ///   complete.
  /// 2.Indeterminate, when the value is None.
  ///   Indeterminate means that progress is being made without indicating how
  ///   much progress has passed and remains.
  #[declare(default)]
  pub value: Option<f32>,
}

impl Compose for LinearProgress {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    distinct_pipe!($read(this).value.is_some())
      .map(move |determinate| {
        if determinate { Self::determinate(this.clone_watcher()) } else { Self::indeterminate() }
      })
      .into_widget()
  }
}

impl LinearProgress {
  fn determinate(this: impl StateWatcher<Value = Self>) -> Widget<'static> {
    flex! {
      align_items: Align::Center,
      class: LINEAR_PROGRESS,
      @Expanded {
        flex: distinct_pipe! { $read(this).value.unwrap_or(1.) },
        @Container {
          size: Size::new(0., 6.),
          class: LINEAR_DETERMINATE_INDICATOR,
        }
      }
      @Expanded {
        flex: distinct_pipe! {$read(this).value.map_or(0., |v| 1. - v) },
        @Container {
          size: Size::new(0., 6.),
          class: LINEAR_DETERMINATE_TRACK,
        }
      }
    }
    .into_widget()
  }

  fn indeterminate() -> Widget<'static> {
    container! {
      class: LINEAR_PROGRESS_INDETERMINATE,
      size: Size::new(0., 6.),
    }
    .into_widget()
  }
}

impl Compose for SpinnerProgress {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut spinner = @SpinnerArc {
        start: Angle::zero(),
        end: distinct_pipe! { Angle::two_pi() * $read(this).value.unwrap_or(0.) },
      };
      // It is essential to ensure that the spinner is accessible by the class,
      // as the class may need to perform animations on the spinner.
      @Providers {
        providers: [Provider::new(spinner.clone_writer())],
        @(spinner) {
          class: distinct_pipe! {
            if $read(this).value.is_some() {
              SPINNER_DETERMINATE
            } else {
              SPINNER_INDETERMINATE
            }
          }
        }
      }
    }
    .into_widget()
  }
}
#[derive(Declare)]
pub struct SpinnerArc {
  pub start: Angle,
  pub end: Angle,
}

impl Render for SpinnerArc {
  fn size_affected_by_child(&self) -> bool { false }
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    clamp.clamp(Size::splat(40.))
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let size = ctx.box_size()?;
    Some(Rect::from_size(size))
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let Self { start, end } = *self;
    let size = ctx.box_size().unwrap();
    if self.offset_angle().to_degrees().abs() < 0.1 || size.is_empty() {
      return;
    }

    let start = start - Angle::pi() / 2.;
    let end = end - Angle::pi() / 2.;
    let center = Point::new(size.width / 2., size.height / 2.);
    let radius = center.x.min(center.y);

    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let painter = ctx.painter();
    match style {
      Some(PaintingStyle::Stroke(strokes)) => {
        let radius = radius - strokes.width / 2.0;
        painter
          .set_strokes(strokes)
          .begin_path(arc_start_at(start, center, radius))
          .arc_to(center, radius, start, end)
          .end_path(false)
          .stroke();
      }
      _ => {
        painter
          .begin_path(center)
          .line_to(arc_start_at(start, center, radius))
          .arc_to(center, radius, start, end)
          .line_to(center)
          .end_path(true)
          .fill();
      }
    }
  }
}

/// A SpinnerArc is a widget of SpinnerProgress.
///
/// This widget expands to its maximum size and utilizes painting styles to fill
/// or stroke the spinner arc. The theme designer can employ `clamp` to restrict
/// the size of this spinner, and `painting_style` and `foreground` to manage
/// the style and color of the spinner.
impl SpinnerArc {
  pub fn offset_angle(&self) -> Angle { self.end - self.start }
}

fn arc_start_at(start: Angle, center: Point, radius: f32) -> Point {
  let radians = start.radians;
  let x = center.x + radius * radians.cos();
  let y = center.y + radius * radians.sin();
  Point::new(x, y)
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    progress_widget,
    WidgetTester::new(self::column! {
      justify_content: JustifyContent::SpaceAround,
      align_items: Align::Center,
      @LinearProgress { value: None }
      @LinearProgress { value: Some(0.2) }
      @Row {
        justify_content: JustifyContent::SpaceAround,
        @SpinnerProgress { value: None }
        @SpinnerProgress { value: Some(0.4) }
      }
    })
    .with_wnd_size(Size::new(300., 200.))
    .with_comparison(0.002)
  );
}
