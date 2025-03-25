use std::{mem::swap, ops::Range};

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

/// The widget displays a slider.
#[derive(Declare)]
pub struct Slider {
  /// The value of the slider
  pub value: f32,

  /// The maximum value of the slider
  #[declare(default = 100.)]
  pub max: f32,

  /// The minimum value of the slider
  #[declare(default = 0.)]
  pub min: f32,

  /// The number of divisions
  ///
  /// if None, the slider will be continuous
  /// if Some(divisions), the slider will be divided into `divisions + 1` parts,
  /// and the indicator will be located to the closest division
  #[declare(default)]
  pub divisions: Option<usize>,
}

impl Slider {
  fn set_to(&mut self, mut v: f32) {
    v = v.clamp(0., 1.);
    if let Some(divisions) = self.divisions {
      if divisions > 0 {
        v = (v * divisions as f32).round() / (divisions as f32);
      }
    }

    self.value = (self.min + v * (self.max - self.min)).clamp(self.min, self.max);
  }

  fn ratio(&self) -> f32 {
    if self.max == self.min {
      return 1.;
    }
    let mut v = (self.value - self.min) / (self.max - self.min);
    v = v.clamp(0., 1.);
    if let Some(divisions) = self.divisions {
      if divisions > 0 {
        v = (v * divisions as f32).round() / (divisions as f32)
      }
    }
    v
  }

  fn validate(&mut self) {
    if self.max < self.min {
      swap(&mut self.max, &mut self.min);
    }

    if self.value < self.min {
      self.value = self.min;
    }

    if self.value > self.max {
      self.value = self.max;
    }
  }

  fn stop_indicator_track(&self) -> Option<impl FnOnce() -> Widget<'static>> {
    let divisions = self.divisions?;
    if divisions == 0 {
      return None;
    }
    let active = (self.ratio() * divisions as f32) as usize;
    Some(fn_widget! {
        stop_indicator_track(divisions + 1, 0..active, vec![active])
    })
  }
}

fn precision(min: f32, max: f32) -> usize {
  ((max - min).log10().floor() - 2.).min(-2.).abs() as usize
}

impl Compose for Slider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let u = this.modifies().subscribe(move |_| {
        let mut this = $this.write();
        this.validate();
        this.forget_modifies();
      });

      let mut row = @Row { align_items: Align::Center };
      let drag_info = Stateful::new(None);
      @ Stack {
        class: SLIDER_CONTAINER,
        @ $row {
          v_align: VAlign::Center,
          on_tap: move |e| {
            let width = $row.layout_size().width;
            $this.write().set_to(e.position().x / width);
          },
          on_disposed: move |_| u.unsubscribe(),
          @Expanded {
            flex: pipe!($this.ratio()),
            @Void { class: SLIDER_ACTIVE_TRACK }
          }
          @Void {
            class: SLIDER_INDICATOR ,
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$drag_info.write() = Some((handle, e.global_pos().x, $this.ratio()));
              }
            },
            on_pointer_move: move|e| if let Some((_, pos, ratio)) = $drag_info.as_ref() {
              let width = $row.layout_size().width;
              let val = ratio + (e.global_pos().x - pos) / width;
              $this.write().set_to(val);
            },
            on_pointer_up: move |_| {
              $drag_info.write().take();
            },
            tooltips: pipe!($this.value).map(move |v| {
              let precision = precision($this.min, $this.max);
              format!("{:.1$}", v, precision)
            }),
          }
          @Expanded {
            flex: pipe!($this.ratio()).map(|v| 1. - v),
            @Void { class: SLIDER_INACTIVE_TRACK }
          }
        }

        @{ pipe!($this.stop_indicator_track()) }
      }
    }
    .into_widget()
  }
}

/// A widget that display a range slider.
#[derive(Declare)]
pub struct RangeSlider {
  /// The start value of the range slider
  pub start: f32,

  /// The end value of the range slider
  pub end: f32,

  /// The maximum value of the range slider
  #[declare(default = 100.)]
  pub max: f32,

  /// The minimum value of the range slider
  #[declare(default = 0.)]
  pub min: f32,

  /// The number of divisions
  ///
  /// if None, the slider will be continuous
  /// if Some(divisions), the slider will be divided into `divisions + 1` parts,
  /// and the indicator will be located to the closest division
  #[declare(default)]
  pub divisions: Option<usize>,
}

impl RangeSlider {
  fn set_ratio(&mut self, mut ratio: f32) {
    ratio = ratio.clamp(0., 1.);
    let val = self.convert_ratio(ratio);
    if (self.start - val).abs() < (self.end - val).abs() {
      self.start = val;
    } else {
      self.end = val;
    }
  }

  fn set_start_ratio(&mut self, ratio: f32) {
    self.start = self
      .convert_ratio(ratio)
      .min(self.end)
      .max(self.min);
  }

  fn set_end_ratio(&mut self, ratio: f32) {
    self.end = self
      .convert_ratio(ratio)
      .max(self.start)
      .min(self.max);
  }

  fn convert_ratio(&self, mut ratio: f32) -> f32 {
    if let Some(divisions) = self.divisions {
      if divisions > 1 {
        ratio = (ratio * divisions as f32).round() / (divisions as f32);
      }
    }
    self.min + ratio * (self.max - self.min)
  }

  fn ratio(&self, v: f32) -> f32 {
    if self.max == self.min {
      return 1.;
    }
    let mut v = (v - self.min) / (self.max - self.min);
    v = v.clamp(0., 1.);
    if let Some(divisions) = self.divisions {
      if divisions > 0 {
        v = (v * divisions as f32).round() / (divisions as f32);
      }
    }
    v
  }

  fn start_ratio(&self) -> f32 { self.ratio(self.start) }

  fn end_ratio(&self) -> f32 { self.ratio(self.end) }

  fn validate(&mut self) {
    if self.max < self.min {
      swap(&mut self.max, &mut self.min);
    }

    if self.start > self.end {
      swap(&mut self.start, &mut self.end);
    }

    if self.start < self.min {
      self.start = self.min;
    }

    if self.end > self.max {
      self.end = self.max;
    }
  }

  fn stop_indicator_track(&self) -> Option<impl FnOnce() -> Widget<'static> + 'static> {
    let divisions = self.divisions?;
    if divisions == 0 {
      return None;
    }
    let start = (self.start_ratio() * divisions as f32) as usize;
    let end = (self.end_ratio() * divisions as f32) as usize;
    Some(fn_widget! {
      stop_indicator_track(divisions + 1, start..end + 1, vec![start, end])
    })
  }
}

impl Compose for RangeSlider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let u = this.modifies().subscribe(move |_| {
        let mut this = $this.write();
        this.validate();
        this.forget_modifies();
      });

      let mut row = @Row { align_items: Align::Center };
      let drag_info1 = Stateful::new(None);
      let drag_info2 = Stateful::new(None);
      @Stack {
        class: SLIDER_CONTAINER,
        @ $row {
          v_align: VAlign::Center,
          on_tap: move |e| {
            let width = $row.layout_size().width;
            $this.write().set_ratio(e.position().x / width);
          },
          on_disposed: move |_| u.unsubscribe(),
          @Expanded {
            flex: pipe!($this.start_ratio()),
            @Void { class: RANGE_SLIDER_INACTIVE_TRACK_LEFT }
          }
          @Void {
            class: SLIDER_INDICATOR,
            tooltips: pipe!($this.start).map(move |v| {
              let precision = precision($this.min, $this.max);
              format!("{:.1$}", v, precision)
            }),
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$drag_info1.write() = Some((handle, e.global_pos().x, $this.start_ratio()));
              }
            },
            on_pointer_move: move |e| {
              if let Some((_, pos, ratio)) = $drag_info1.as_ref() {
                let width = $row.layout_size().width;
                let val = ratio + (e.global_pos().x - pos) / width;
                $this.write().set_start_ratio(val);
              }
            },
            on_pointer_up: move |_| { $drag_info1.write().take(); }
          }
          @Expanded {
            flex: pipe!($this.end_ratio() - $this.start_ratio()),
            @Void { class: RANGE_SLIDER_ACTIVE_TRACK }
          }
          @Void {
            class: SLIDER_INDICATOR,
            tooltips: pipe!($this.end).map(move |v| {
              let precision = precision($this.min, $this.max);
              format!("{:.1$}", v, precision)
            }),
            on_tap: move |e| e.stop_propagation(),
            on_pointer_down: move |e| {
              if let Some(handle) = GrabPointer::grab(e.current_target(), &e.window()) {
                *$drag_info2.write() = Some((handle, e.global_pos().x, $this.end_ratio()));
              }
            },
            on_pointer_move: move |e| {
              if let Some((_, pos, ratio)) = $drag_info2.as_ref() {
                let width = $row.layout_size().width;
                let val = ratio + (e.global_pos().x - pos) / width;
                $this.write().set_end_ratio(val);
              }
            },
            on_pointer_up: move |_| { $drag_info2.write().take(); }
          }
          @Expanded {
            flex: pipe!($this.end_ratio()).map(|v| 1. - v),
            @Void { class: RANGE_SLIDER_INACTIVE_TRACK_RIGHT }
          }
        }
        @{ pipe!($this.stop_indicator_track()) }
      }
    }
    .into_widget()
  }
}

fn stop_indicator_track(cnt: usize, actives: Range<usize>, filter: Vec<usize>) -> Widget<'static> {
  fn_widget!(
    @IgnorePointer {
      @Row {
        v_align: VAlign::Center,
        align_items: Align::Center,
        justify_content: JustifyContent::SpaceBetween,
        @ {
          (0..cnt).map(move |i| {
            @Void {
              class: if actives.contains(&i) {
                STOP_INDICATOR_ACTIVE
              } else {
                STOP_INDICATOR_INACTIVE
              },
              visible: !filter.contains(&i),
            }
          })
        }
      }
    }
  )
  .into_widget()
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    slider_widgets,
    WidgetTester::new(self::column! {
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
}
