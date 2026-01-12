use ribir_core::prelude::*;

use crate::prelude::*;

class_names! {
  #[doc = "Class name for the slider container"]
  SLIDER_CONTAINER,
  #[doc = "Class name for the slider thumb container"]
  SLIDER_THUMB_CONTAINER,
  #[doc = "Class name for the slider thumb"]
  SLIDER_THUMB,
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
  #[doc="Class name for the active tick mark"]
  SLIDER_TICK_ACTIVE,
  #[doc="Class name for the inactive tick mark"]
  SLIDER_TICK_INACTIVE,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderChanged {
  pub from: f32,
  pub to: f32,
}

pub type SliderChangedEvent = CustomEvent<SliderChanged>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeSliderValue {
  pub start: f32,
  pub end: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RangeSliderChanged {
  pub from: RangeSliderValue,
  pub to: RangeSliderValue,
}

pub type RangeSliderChangedEvent = CustomEvent<RangeSliderChanged>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderTicks {
  #[default]
  None,
  Always,
}

trait SliderCore {
  fn min(&self) -> f32;
  fn max(&self) -> f32;
  fn divisions(&self) -> Option<usize>;

  fn snap_v(&self, v: f32) -> f32 {
    let (min, max) = (self.min(), self.max());
    let v = v.clamp(min.min(max), min.max(max));
    if let Some(divisions) = self.divisions()
      && max != min
    {
      let ratio = (v - min) / (max - min);
      let ratio = (ratio * divisions as f32).round() / (divisions as f32);
      min + ratio * (max - min)
    } else {
      v
    }
  }

  fn calc_ratio(&self, v: f32) -> f32 {
    let (min, max) = (self.min(), self.max());
    if max == min { 1. } else { ((v - min) / (max - min)).clamp(0., 1.) }
  }

  fn convert_ratio(&self, ratio: f32) -> f32 {
    self.min() + ratio.clamp(0., 1.) * (self.max() - self.min())
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RangeSliderPart {
  Start,
  End,
}

fn slider_update<E: std::ops::Deref<Target = CommonEvent>>(
  this: &impl StateWriter<Value = Slider>, e: &E, ratio: f32,
) {
  let mut this = this.write();
  let from = this.value;
  this.set_by_ratio(ratio);
  let to = this.value;
  if from != to {
    e.window()
      .bubble_custom_event(e.target(), SliderChanged { from, to });
  }
}

fn range_slider_update<E: std::ops::Deref<Target = CommonEvent>>(
  this: &impl StateWriter<Value = RangeSlider>, e: &E, ratio: f32, part: RangeSliderPart,
) {
  let from = this.read().value();
  {
    let mut writer = this.write();
    match part {
      RangeSliderPart::Start => writer.set_start_ratio(ratio),
      RangeSliderPart::End => writer.set_end_ratio(ratio),
    }
  }
  let to = this.read().value();
  if from != to {
    e.window()
      .bubble_custom_event(e.target(), RangeSliderChanged { from, to });
  }
}

fn slider_core_validate(min: &mut f32, max: &mut f32, divisions: &mut Option<usize>) {
  if *min > *max {
    std::mem::swap(min, max);
  }
  if let Some(0) = *divisions {
    *divisions = None;
  }
}

fn slider_tooltip(min: f32, max: f32, val: f32) -> String {
  let precision = ((max - min).log10().floor() - 2.).min(-2.).abs() as usize;
  format!("{:.1$}", val, precision)
}

fn slider_ticks(
  divisions: usize, range: std::ops::RangeInclusive<usize>,
  is_hide: impl Fn(usize) -> bool + 'static,
) -> Widget<'static> {
  fn_widget! {
    @Flex {
      v_align: VAlign::Center,
      align_items: Align::Center,
      justify_content: JustifyContent::SpaceBetween,
      @ {
        (0..=divisions).map(move |i| {
          @Void {
            class: if range.contains(&i) { SLIDER_TICK_ACTIVE } else { SLIDER_TICK_INACTIVE },
            opacity: if is_hide(i) { 0. } else { 1. },
          }
        })
      }
    }
  }
  .into_widget()
}
/// The widget displays a slider.
#[declare(validate)]
pub struct Slider {
  /// The value of the slider
  #[declare(setter = set_value, event = SliderChanged.to)]
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
    slider_core_validate(&mut self.min, &mut self.max, &mut self.divisions);
    self.set_value(self.value);
    Ok(self)
  }
}

impl SliderCore for Slider {
  fn min(&self) -> f32 { self.min }
  fn max(&self) -> f32 { self.max }
  fn divisions(&self) -> Option<usize> { self.divisions }
}

impl Slider {
  pub fn value(&self) -> f32 { self.value }

  pub fn max(&self) -> f32 { self.max }

  pub fn min(&self) -> f32 { self.min }

  pub fn divisions(&self) -> Option<usize> { self.divisions }

  fn set_by_ratio(&mut self, ratio: f32) { self.set_value(self.convert_ratio(ratio)); }

  fn ratio(&self) -> f32 { self.calc_ratio(self.value) }

  pub fn set_value(&mut self, val: f32) { self.value = self.snap_v(val); }

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
    self.set_value(self.value);
  }

  fn thumb_container(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let ticks = Provider::of::<SliderTicks>(BuildCtx::get()).map(|v| *v).unwrap_or_default();
      let mut thumb_container = @Stack { class: SLIDER_THUMB_CONTAINER };
      let thumb_container_width = thumb_container.layout_width();
      let mut thumb = @Void {
        class: SLIDER_THUMB,
        tooltips: pipe! {
          let this = $read(this);
          slider_tooltip(this.min, this.max, this.value)
        },
      };
      let thumb_width = thumb.layout_width();
      @ (thumb_container) {
        h_align: HAlign::Stretch,
        @pipe! {
          (ticks == SliderTicks::Always).then(|| {
            let this = $read(this);
            let divisions = this.divisions?;
            let active = (this.ratio() * divisions as f32).round() as usize;
            Some(slider_ticks(divisions, 0..=active, move |i| i == active))
          }).flatten()
        }
        @(thumb) {
          anchor: pipe! {
            let ratio = $read(this).ratio();
            let track_width = *$read(thumb_container_width);
            let thumb_width = *$read(thumb_width);
            Anchor::left(ratio * track_width - thumb_width / 2.)
          }
        }
      }
    }
    .into_widget()
  }
}

impl Compose for Slider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut track = @Flex { align_items: Align::Center };
      let track_width = track.layout_width();
      @PointerSelectRegion {
        on_custom: move |e: &mut PointerSelectEvent| {
          let width = *$read(track_width);
          if width > 0. {
            let (_, to) = e.data().endpoints();
            slider_update(&$writer(this), e, to.x / width);
          }
        },
        @Stack {
          class: SLIDER_CONTAINER,
          @(track) {
            v_align: VAlign::Center,
            @Expanded {
              flex: pipe!($read(this).ratio()),
              @Void { class: SLIDER_ACTIVE_TRACK }
            }
            @Expanded {
              flex: pipe!(1. - $read(this).ratio()),
              @Void { class: SLIDER_INACTIVE_TRACK }
            }
          }
          @InParentLayout {
            @Slider::thumb_container($writer(this))
          }
        }
      }
    }
    .into_widget()
  }
}

/// A widget that display a range slider.
#[declare(validate)]
pub struct RangeSlider {
  /// The start value of the range slider
  #[declare(setter = set_start, event = RangeSliderChanged.to.start)]
  start: f32,

  /// The end value of the range slider
  #[declare(setter = set_end, event = RangeSliderChanged.to.end)]
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
    slider_core_validate(&mut self.min, &mut self.max, &mut self.divisions);
    self.start = self.snap_v(self.start);
    self.end = self.snap_v(self.end);
    if self.start > self.end {
      std::mem::swap(&mut self.start, &mut self.end);
    }
    Ok(self)
  }

  fn thumb_container(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let ticks = Provider::of::<SliderTicks>(BuildCtx::get()).map(|v| *v).unwrap_or_default();
      let mut thumb_container = @Stack { class: SLIDER_THUMB_CONTAINER };
      let thumb_container_width = thumb_container.layout_width();

      let mut start_thumb = @Void {
        class: SLIDER_THUMB,
        tooltips: pipe! {
          let this = $read(this);
          slider_tooltip(this.min, this.max, this.start)
        },
      };
      let start_thumb_width = start_thumb.layout_width();
      let mut end_thumb = @Void {
        class: SLIDER_THUMB,
        tooltips: pipe! {
          let this = $read(this);
          slider_tooltip(this.min, this.max, this.end)
        },
      };
      let end_thumb_width = end_thumb.layout_width();

      @(thumb_container) {
        h_align: HAlign::Stretch,
        @pipe! {
          (ticks == SliderTicks::Always).then(|| {
            let this = $read(this);
            let divisions = this.divisions?;
            let start = (this.start_ratio() * divisions as f32).round() as usize;
            let end = (this.end_ratio() * divisions as f32).round() as usize;
            Some(slider_ticks(divisions, start..=end, move |i| i == start || i == end))
          }).flatten()
        }
        @(start_thumb) {
          anchor: pipe! {
            let ratio = $read(this).start_ratio();
            let track_width = *$read(thumb_container_width);
            let thumb_width = *$read(start_thumb_width);
            Anchor::left(ratio * track_width - thumb_width / 2.)
          }
        }
        @(end_thumb) {
          anchor: pipe! {
            let ratio = $read(this).end_ratio();
            let track_width = *$read(thumb_container_width);
            let thumb_width = *$read(end_thumb_width);
            Anchor::left(ratio * track_width - thumb_width / 2.)
          }
        }
      }
    }
    .into_widget()
  }
}

impl RangeSlider {
  pub fn start(&self) -> f32 { self.start }

  pub fn end(&self) -> f32 { self.end }

  pub fn max(&self) -> f32 { self.max }

  pub fn min(&self) -> f32 { self.min }

  pub fn divisions(&self) -> Option<usize> { self.divisions }

  pub fn value(&self) -> RangeSliderValue { RangeSliderValue { start: self.start, end: self.end } }

  pub fn set_start(&mut self, start: f32) {
    self.start = self.snap_v(start.clamp(self.min, self.end.max(self.min)));
  }

  pub fn set_end(&mut self, end: f32) {
    self.end = self.snap_v(end.clamp(self.start.min(self.max), self.max));
  }

  pub fn set_max(&mut self, max: f32) {
    self.max = max.max(self.min);
    self.set_end(self.end);
    self.set_start(self.start);
  }

  pub fn set_min(&mut self, min: f32) {
    self.min = min.min(self.max);
    self.set_start(self.start);
    self.set_end(self.end);
  }

  pub fn set_divisions(&mut self, divisions: Option<usize>) {
    self.divisions = divisions.filter(|&v| v > 0);
    self.set_start(self.start);
    self.set_end(self.end);
  }

  fn choose_part(&self, ratio: f32) -> RangeSliderPart {
    let val = self.snap_v(self.convert_ratio(ratio));
    if (self.start - val).abs() < (self.end - val).abs() {
      RangeSliderPart::Start
    } else if (self.start - val).abs() > (self.end - val).abs() {
      RangeSliderPart::End
    } else if ratio < self.start_ratio() {
      RangeSliderPart::Start
    } else {
      RangeSliderPart::End
    }
  }

  fn set_start_ratio(&mut self, ratio: f32) { self.set_start(self.convert_ratio(ratio)); }

  fn set_end_ratio(&mut self, ratio: f32) { self.set_end(self.convert_ratio(ratio)); }

  fn start_ratio(&self) -> f32 { self.calc_ratio(self.start) }

  fn end_ratio(&self) -> f32 { self.calc_ratio(self.end) }
}

impl SliderCore for RangeSlider {
  fn min(&self) -> f32 { self.min }
  fn max(&self) -> f32 { self.max }
  fn divisions(&self) -> Option<usize> { self.divisions }
}

impl Compose for RangeSlider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut track = @Flex { align_items: Align::Center };
      let track_width = track.layout_width();
      let active_part = Stateful::new(RangeSliderPart::Start);

      @PointerSelectRegion {
        on_custom: move |e: &mut PointerSelectEvent| {
          let width = *$read(track_width);
          if width <= 0. { return; }
          let (_, to) = e.data().endpoints();
          let mut active_part = $write(active_part);
          if let PointerSelectData::Start(p) = e.data() {
            let ratio = p.x / width;
            *active_part = $read(this).choose_part(ratio);
          }
          range_slider_update(&$writer(this), e, to.x / width, *active_part);
        },
        @Stack {
          class: SLIDER_CONTAINER,
          @(track) {
            v_align: VAlign::Center,
            @Expanded {
              flex: pipe!($read(this).start_ratio()),
              @Void { class: RANGE_SLIDER_INACTIVE_TRACK_LEFT }
            }
            @Expanded {
              flex: pipe! {
                let this = $read(this);
                this.end_ratio() - this.start_ratio()
              },
              @Void { class: RANGE_SLIDER_ACTIVE_TRACK }
            }
            @Expanded {
              flex: pipe!(1. - $read(this).end_ratio()),
              @Void { class: RANGE_SLIDER_INACTIVE_TRACK_RIGHT }
            }
          }
          @InParentLayout {
            @RangeSlider::thumb_container($writer(this))
          }
        }
      }
    }
    .into_widget()
  }
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

  widget_image_tests!(
    slider_ticks_widgets,
    WidgetTester::new(providers! {
      providers: [Provider::new(SliderTicks::Always)],
      @flex! {
        direction: Direction::Vertical,
        justify_content: JustifyContent::SpaceAround,
        align_items: Align::Center,
        @Slider { value: 32. }
        @Slider { value: 32., divisions: Some(10) }
        @RangeSlider { start: 10., end: 73. }
        @RangeSlider { start: 10., end: 73., divisions: Some(10) }
      }
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

  #[test]
  fn range_slider_set_by_ratio_robustness() {
    let mut range = RangeSlider { start: 50., end: 50., min: 0., max: 100., divisions: None };

    // Move start (click left)
    let ratio = 0.4;
    let part = range.choose_part(ratio);
    assert_eq!(part, RangeSliderPart::Start);
    range.set_start_ratio(ratio);
    assert!((range.start() - 40.).abs() < 1e-5);
    assert!((range.end() - 50.).abs() < 1e-5);

    // Reset
    range.set_start(50.);

    // Move end (click right)
    let ratio = 0.6;
    let part = range.choose_part(ratio);
    assert_eq!(part, RangeSliderPart::End);
    range.set_end_ratio(ratio);
    assert!((range.start() - 50.).abs() < 1e-5);
    assert!((range.end() - 60.).abs() < 1e-5);
  }
  #[test]
  fn slider_click_update() {
    reset_test_env!();
    let (value, w_value) = split_value(0.);
    let w = fn_widget! {
      let slider = @Slider { value: 0., max: 100. };
      watch!($read(slider).value())
        .subscribe(move |v| *$write(w_value) = v);
      @SizedBox {
        size: Size::new(100., 20.),
        @ { slider }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 20.));
    wnd.draw_frame();

    wnd.process_cursor_move(Point::new(50., 10.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    assert!((*value.read() - 50.).abs() < 1e-5);
  }

  #[test]
  fn range_slider_click_update() {
    reset_test_env!();
    let (value, w_value) = split_value(RangeSliderValue { start: 0., end: 100. });
    let w = fn_widget! {
      let slider = @RangeSlider { start: 10., end: 90., max: 100. };
      watch!($read(slider).value())
        .subscribe(move |v| *$write(w_value) = v);
      @SizedBox {
        size: Size::new(100., 20.),
        @ { slider }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 20.));
    wnd.draw_frame();

    // Click at 20% (nearer to start)
    wnd.process_cursor_move(Point::new(20., 10.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    let value = value.read();
    assert!((value.start - 20.).abs() < 1e-5);
    assert!((value.end - 90.).abs() < 1e-5);
  }
}
