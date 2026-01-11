use ribir_core::prelude::*;

use crate::prelude::*;

class_names! {
  #[doc = "Class name for the slider container"]
  SLIDER_CONTAINER,
  #[doc = "Class name for the slider indicator"]
  SLIDER_INDICATOR,
  #[doc = "Class name for the slider track"]
  SLIDER_ACTIVE_TRACK,
  #[doc = "Class name for the slider inactive track"]
  SLIDER_INACTIVE_TRACK,
  #[doc = "Class name for the left inactive track of range slider"]
  RANGE_SLIDER_INACTIVE_TRACK_LEFT,
  #[doc = "Class name for the right inactive track of range slider"]
  RANGE_SLIDER_INACTIVE_TRACK_RIGHT,
  #[doc = "Class name for the active track of range slider"]
  RANGE_SLIDER_ACTIVE_TRACK,
  #[doc="Class name for the active stop indicator"]
  STOP_INDICATOR_ACTIVE,
  #[doc="Class name for the inactive stop indicator"]
  STOP_INDICATOR_INACTIVE,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderChanged {
  pub from: f32,
  pub to: f32,
}

pub type SliderChangedEvent = CustomEvent<SliderChanged>;
/// The widget displays a slider.
#[declare(validate)]
pub struct Slider {
  /// The value of the slider
  #[declare(setter = set_value)]
  value: f32,

  /// The maximum value of the slider
  #[declare(default = 100., setter = set_max)]
  max: f32,

  /// The minimum value of the slider
  #[declare(default = 0., setter = set_min)]
  min: f32,

  /// The number of divisions
  ///
  /// if None, the slider will be continuous
  /// if Some(divisions), the slider will be divided into `divisions + 1` parts,
  /// and the indicator will be located to the closest division
  #[declare(default, setter = set_divisions)]
  divisions: Option<usize>,
}

impl Slider {
  fn declare_validate(mut self) -> Result<Self, std::convert::Infallible> {
    if self.min > self.max {
      std::mem::swap(&mut self.min, &mut self.max);
    }
    self.value = self.value.clamp(self.min, self.max);
    if let Some(0) = self.divisions {
      self.divisions = None;
    }
    self.value = self.snap_v(self.value);
    Ok(self)
  }
}

impl Slider {
  pub fn value(&self) -> f32 { self.value }

  pub fn max(&self) -> f32 { self.max }

  pub fn min(&self) -> f32 { self.min }

  pub fn divisions(&self) -> Option<usize> { self.divisions }

  fn set_by_ratio(&mut self, mut v: f32) {
    v = v.clamp(0., 1.);
    self.value = self.snap_v(self.min + v * (self.max - self.min));
  }

  fn ratio(&self) -> f32 {
    if self.max == self.min {
      return 1.;
    }
    ((self.value - self.min) / (self.max - self.min)).clamp(0., 1.)
  }

  fn snap_v(&self, v: f32) -> f32 {
    if let Some(divisions) = self.divisions
      && self.max != self.min
    {
      let ratio = (v - self.min) / (self.max - self.min);
      let ratio = (ratio * divisions as f32).round() / (divisions as f32);
      self.min + ratio * (self.max - self.min)
    } else {
      v
    }
  }

  pub fn set_value(&mut self, val: f32) { self.value = self.snap_v(val.clamp(self.min, self.max)); }

  pub fn set_max(&mut self, max: f32) {
    self.max = max.max(self.min);
    self.set_value(self.value);
  }

  pub fn set_min(&mut self, min: f32) {
    self.min = min.min(self.max);
    self.set_value(self.value);
  }

  pub fn set_divisions(&mut self, divisions: Option<usize>) {
    self.divisions = divisions.filter(|&v| v > 0);
    self.value = self.snap_v(self.value);
  }

  fn stop_indicator_track(&self) -> Option<BoxFnWidget<'static>> {
    let divisions = self.divisions?;
    let active = (self.ratio() * divisions as f32).round() as usize;
    Some(stop_indicator_track(divisions + 1, 0..=active, vec![active]))
  }
}

fn precision(min: f32, max: f32) -> usize {
  ((max - min).log10().floor() - 2.).min(-2.).abs() as usize
}

impl Compose for Slider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut row = @Flex { align_items: Align::Center };
      let drag_info = Stateful::new(None);
      @Stack {
        class: SLIDER_CONTAINER,
        @(row) {
          v_align: VAlign::Center,
          on_tap: move |e| {
            let width = *$read(row.layout_width());
            let old = $read(this).value;
            $write(this).set_by_ratio(e.position().x / width);
            if old != $read(this).value {
              e.window().bubble_custom_event(e.current_target(), SliderChanged {
                from: old,
                to: $read(this).value,
              });
            }
          },
          @Expanded {
            flex: pipe!($read(this).ratio()),
            @Void { class: SLIDER_ACTIVE_TRACK }
          }
          @Void {
            class: SLIDER_INDICATOR ,
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$write(drag_info) = Some((handle, e.global_pos().x, $read(this).ratio()));
              }
            },
            on_pointer_move: move|e| if let Some((_, pos, ratio)) = $read(drag_info).as_ref() {
              let width = *$read(row.layout_width());
              let val = ratio + (e.global_pos().x - pos) / width;
              let old = $read(this).value;
              $write(this).set_by_ratio(val);
              if old != $read(this).value {
                e.window().bubble_custom_event(e.current_target(), SliderChanged {
                  from: old,
                  to: $read(this).value,
                });
              }
            },
            on_pointer_up: move |_| {
              $write(drag_info).take();
            },
            tooltips: pipe! {
              let this = $read(this);
              let precision = precision(this.min, this.max);
              format!("{:.1$}", this.value, precision)
            },
          }
          @Expanded {
            flex: pipe!(1. - $read(this).ratio()),
            @Void { class: SLIDER_INACTIVE_TRACK }
          }
        }

        @{ pipe!($read(this).stop_indicator_track() ) }
      }
    }
    .into_widget()
  }
}

/// A widget that display a range slider.
#[declare(validate)]
pub struct RangeSlider {
  /// The start value of the range slider
  #[declare(setter = set_start)]
  start: f32,

  /// The end value of the range slider
  #[declare(setter = set_end)]
  end: f32,

  /// The maximum value of the range slider
  #[declare(default = 100., setter = set_max)]
  max: f32,

  /// The minimum value of the range slider
  #[declare(default = 0., setter = set_min)]
  min: f32,

  /// The number of divisions
  ///
  /// if None, the slider will be continuous
  /// if Some(divisions), the slider will be divided into `divisions + 1` parts,
  /// and the indicator will be located to the closest division
  #[declare(default, setter = set_divisions)]
  divisions: Option<usize>,
}

impl RangeSlider {
  fn declare_validate(mut self) -> Result<Self, std::convert::Infallible> {
    if self.min > self.max {
      std::mem::swap(&mut self.min, &mut self.max);
    }
    self.start = self.start.clamp(self.min, self.max);
    self.end = self.end.clamp(self.min, self.max);
    if self.start > self.end {
      std::mem::swap(&mut self.start, &mut self.end);
    }
    if let Some(0) = self.divisions {
      self.divisions = None;
    }
    self.start = self.snap_v(self.start);
    self.end = self.snap_v(self.end);
    Ok(self)
  }
}

impl RangeSlider {
  pub fn start(&self) -> f32 { self.start }

  pub fn end(&self) -> f32 { self.end }

  pub fn max(&self) -> f32 { self.max }

  pub fn min(&self) -> f32 { self.min }

  pub fn divisions(&self) -> Option<usize> { self.divisions }

  fn set_by_ratio(&mut self, mut ratio: f32) {
    ratio = ratio.clamp(0., 1.);
    let val = self.snap_v(self.convert_ratio(ratio));
    if (self.start - val).abs() < (self.end - val).abs() {
      self.set_start(val);
    } else {
      self.set_end(val);
    }
  }

  fn set_start_ratio(&mut self, ratio: f32) {
    let val = self.convert_ratio(ratio.clamp(0., 1.));
    self.set_start(val);
  }

  fn set_end_ratio(&mut self, ratio: f32) {
    let val = self.convert_ratio(ratio.clamp(0., 1.));
    self.set_end(val);
  }

  fn convert_ratio(&self, ratio: f32) -> f32 { self.min + ratio * (self.max - self.min) }

  fn ratio(&self, v: f32) -> f32 {
    if self.max == self.min {
      return 1.;
    }
    ((v - self.min) / (self.max - self.min)).clamp(0., 1.)
  }

  fn start_ratio(&self) -> f32 { self.ratio(self.start) }

  fn end_ratio(&self) -> f32 { self.ratio(self.end) }

  fn snap_v(&self, v: f32) -> f32 {
    if let Some(divisions) = self.divisions
      && self.max != self.min
    {
      let ratio = (v - self.min) / (self.max - self.min);
      let ratio = (ratio * divisions as f32).round() / (divisions as f32);
      self.min + ratio * (self.max - self.min)
    } else {
      v
    }
  }

  pub fn set_start(&mut self, start: f32) {
    self.start = self.snap_v(start.clamp(self.min, self.end));
  }

  pub fn set_end(&mut self, end: f32) { self.end = self.snap_v(end.clamp(self.start, self.max)); }

  pub fn set_max(&mut self, max: f32) {
    self.max = max.max(self.min);
    self.end = self.end.min(self.max);
    self.start = self.start.min(self.end);
    self.start = self.snap_v(self.start);
    self.end = self.snap_v(self.end);
  }

  pub fn set_min(&mut self, min: f32) {
    self.min = min.min(self.max);
    self.start = self.start.max(self.min);
    self.end = self.end.max(self.start);
    self.start = self.snap_v(self.start);
    self.end = self.snap_v(self.end);
  }

  pub fn set_divisions(&mut self, divisions: Option<usize>) {
    self.divisions = divisions.filter(|&v| v > 0);
    self.start = self.snap_v(self.start);
    self.end = self.snap_v(self.end);
  }

  fn stop_indicator_track(&self) -> Option<BoxFnWidget<'static>> {
    let divisions = self.divisions?;
    let start = (self.start_ratio() * divisions as f32).round() as usize;
    let end = (self.end_ratio() * divisions as f32).round() as usize;
    Some(stop_indicator_track(divisions + 1, start..=end, vec![start, end]))
  }
}

impl Compose for RangeSlider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut row = @Flex { align_items: Align::Center };
      let drag_info1 = Stateful::new(None);
      let drag_info2 = Stateful::new(None);
      @Stack {
        class: SLIDER_CONTAINER,
        @(row) {
          v_align: VAlign::Center,
          on_tap: move |e| {
            let width = *$read(row.layout_width());
            $write(this).set_by_ratio(e.position().x / width);
          },
          @Expanded {
            flex: pipe!($read(this).start_ratio()),
            @Void { class: RANGE_SLIDER_INACTIVE_TRACK_LEFT }
          }
          @Void {
            class: SLIDER_INDICATOR,
            tooltips: pipe!{
              let this = $read(this);
              let precision = precision(this.min, this.max);
              format!("{:.1$}", this.start, precision)
            },
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$write(drag_info1) = Some((handle, e.global_pos().x, $read(this).start_ratio()));
              }
            },
            on_pointer_move: move |e| {
              if let Some((_, pos, ratio)) = $read(drag_info1).as_ref() {
                let width = *$read(row.layout_width());
                let val = ratio + (e.global_pos().x - pos) / width;
                $write(this).set_start_ratio(val);
              }
            },
            on_pointer_up: move |_| { $write(drag_info1).take(); }
          }
          @Expanded {
            flex: pipe!{
              let this = $read(this);
              this.end_ratio() - this.start_ratio()
            },
            @Void { class: RANGE_SLIDER_ACTIVE_TRACK }
          }
          @Void {
            class: SLIDER_INDICATOR,
            tooltips: pipe!{
              let this = $read(this);
              let precision = precision(this.min, this.max);
              format!("{:.1$}", this.end, precision)
            },
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$write(drag_info2) = Some((handle, e.global_pos().x, $read(this).end_ratio()));
              }
            },
            on_pointer_move: move |e| {
              if let Some((_, pos, ratio)) = $read(drag_info2).as_ref() {
                let width = *$read(row.layout_width());
                let val = ratio + (e.global_pos().x - pos) / width;
                $write(this).set_end_ratio(val);
              }
            },
            on_pointer_up: move |_| { $write(drag_info2).take(); }
          }
          @Expanded {
            flex: pipe!(1. - $read(this).end_ratio()),
            @Void { class: RANGE_SLIDER_INACTIVE_TRACK_RIGHT }
          }
        }
        @{ pipe!($read(this).stop_indicator_track()) }
      }
    }
    .into_widget()
  }
}

fn stop_indicator_track(
  cnt: usize, actives: std::ops::RangeInclusive<usize>, filter: Vec<usize>,
) -> BoxFnWidget<'static> {
  fn_widget!(
    let stop_builder = move |i| {
      @Void {
        class: if actives.contains(&i) {
          STOP_INDICATOR_ACTIVE
        } else {
          STOP_INDICATOR_INACTIVE
        },
        visible: !filter.contains(&i),
      }
    };

    // ReWrap FatObj to get the whole stop Widget's layout
    let mut last = FatObj::new(stop_builder(cnt - 1));
    let mut flex =  @Flex {
        align_items: Align::Center,
        justify_content: JustifyContent::SpaceBetween
    };

    @IgnorePointer {
      @(flex) {
        v_align: VAlign::Center,
        opacity: pipe!($read(last.layout_rect()).max_x() <= *$read(flex.layout_width()) + 0.001)
          .map(|v| if v { 1. } else { 0. }),
        @ {(0..cnt-1).map(stop_builder)}
        @{ last }
      }
    }
  )
  .boxed()
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    slider_widgets,
    WidgetTester::new(flex! {
      direction: Direction::Vertical,
      justify_content: JustifyContent::SpaceAround,
      align_items: Align::Center,
      @Slider { value: 32. }
      @Slider { value: 32., divisions: Some(10) }
      @RangeSlider { start: 10., end: 73. }
      @RangeSlider { start: 10., end: 73., divisions: Some(10) }
    })
    .with_wnd_size(Size::new(300., 200.))
    .with_comparison(0.0002)
  );

  #[test]
  fn slider_divisions_calibration() {
    let mut slider = Slider { value: 50., min: 0., max: 100., divisions: Some(0) };
    slider.set_divisions(Some(0));
    assert_eq!(slider.divisions(), None);

    slider.set_value(50.5);
    assert_eq!(slider.value(), 50.5); // Continuous

    slider.set_divisions(Some(10));
    assert_eq!(slider.divisions(), Some(10));
    slider.set_value(32.);
    assert!((slider.value() - 30.).abs() < 1e-5); // Snapped
  }

  #[test]
  fn slider_min_max_clamping() {
    let mut slider = Slider { value: 50., min: 0., max: 100., divisions: None };
    slider.set_min(150.);
    assert_eq!(slider.min(), 100.);
    assert_eq!(slider.max(), 100.);

    slider.set_max(50.);
    assert_eq!(slider.min(), 100.);
    assert_eq!(slider.max(), 100.);

    slider.set_min(0.);
    assert_eq!(slider.min(), 0.);
    assert_eq!(slider.max(), 100.);
  }

  #[test]
  fn range_slider_behavior() {
    let mut range = RangeSlider { start: 10., end: 90., min: 0., max: 100., divisions: Some(10) };
    range.set_start(95.);
    assert!((range.start() - 90.).abs() < 1e-5); // Clamped by end (90)

    range.set_min(120.);
    assert!((range.min() - 100.).abs() < 1e-5);
    assert!((range.start() - 100.).abs() < 1e-5);
    assert!((range.end() - 100.).abs() < 1e-5);
  }

  #[test]
  fn slider_declare_validate() {
    reset_test_env!();
    // 1. Min > Max -> Swap
    let mut builder = Slider::declarer();
    builder.with_value(0.).with_min(100.).with_max(0.);
    let slider = builder.finish();
    assert_eq!(slider.read().min, 0.);
    assert_eq!(slider.read().max, 100.);

    // 2. Value out of range -> Clamp
    let mut builder = Slider::declarer();
    builder.with_value(150.).with_max(100.);
    let slider = builder.finish();
    assert_eq!(slider.read().value, 100.);

    let mut builder = Slider::declarer();
    builder.with_value(-50.).with_min(0.);
    let slider = builder.finish();
    assert_eq!(slider.read().value, 0.);

    // 3. Divisions: Some(0) -> None
    let mut builder = Slider::declarer();
    builder.with_value(0.).with_divisions(Some(0));
    let slider = builder.finish();
    assert_eq!(slider.read().divisions, None);

    // 4. Value snapping
    let mut builder = Slider::declarer();
    builder
      .with_min(0.)
      .with_max(10.)
      .with_divisions(Some(2))
      .with_value(3.);
    let slider = builder.finish();
    assert_eq!(slider.read().value, 5.);

    let mut builder = Slider::declarer();
    builder
      .with_min(0.)
      .with_max(10.)
      .with_divisions(Some(2))
      .with_value(2.);
    let slider = builder.finish();
    assert_eq!(slider.read().value, 0.);
  }

  #[test]
  fn range_slider_declare_validate() {
    reset_test_env!();
    // 1. Min > Max -> Swap
    let mut builder = RangeSlider::declarer();
    builder
      .with_start(0.)
      .with_end(0.)
      .with_min(100.)
      .with_max(0.);
    let range = builder.finish();
    assert_eq!(range.read().min, 0.);
    assert_eq!(range.read().max, 100.);

    // 2. Start/End out of range -> Clamp
    let mut builder = RangeSlider::declarer();
    builder
      .with_start(-10.)
      .with_end(110.)
      .with_min(0.)
      .with_max(100.);
    let range = builder.finish();
    assert_eq!(range.read().start, 0.);
    assert_eq!(range.read().end, 100.);

    // 3. Start > End -> Swap
    let mut builder = RangeSlider::declarer();
    builder.with_start(80.).with_end(20.);
    let range = builder.finish();
    assert_eq!(range.read().start, 20.);
    assert_eq!(range.read().end, 80.);

    // 4. Divisions: Some(0) -> None
    let mut builder = RangeSlider::declarer();
    builder
      .with_start(0.)
      .with_end(0.)
      .with_divisions(Some(0));
    let range = builder.finish();
    assert_eq!(range.read().divisions, None);

    // 5. Snapping
    let mut builder = RangeSlider::declarer();
    builder
      .with_min(0.)
      .with_max(100.)
      .with_divisions(Some(4))
      .with_start(12.)
      .with_end(88.);
    let range = builder.finish();
    assert_eq!(range.read().start, 0.);
    assert_eq!(range.read().end, 100.);
  }
}
