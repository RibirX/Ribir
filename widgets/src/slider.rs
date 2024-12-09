use core::f32;
use std::mem::swap;

use ribir_core::prelude::*;

use crate::layout::{Expanded, Row};

class_names! {
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
  RANGE_SLIDER_ACTIVE_TRACK
}

#[derive(Declare)]
pub struct Slider {
  pub value: f32,
  #[declare(default = 100.)]
  pub max: f32,
  #[declare(default = 0.)]
  pub min: f32,
  #[declare(default)]
  pub divisions: Option<usize>,
}

impl Slider {
  fn set_to(&mut self, mut v: f32) {
    if let Some(divisions) = self.divisions {
      v = (v * divisions as f32).round() / (divisions as f32);
    }

    self.value = (self.min + v * (self.max - self.min)).clamp(self.min, self.max);
  }

  fn ratio(&self) -> f32 {
    if self.max == self.min {
      return 1.;
    }
    let v = (self.value - self.min) / (self.max - self.min);
    v.clamp(0., 1.)
  }

  fn validate(mut this: WriteRef<Self>) {
    if this.max < this.min {
      let Self { max, min, .. } = &mut *this;
      swap(max, min);
    }

    if this.value < this.min {
      this.value = this.min;
    }

    if this.value > this.max {
      this.value = this.max;
    }
  }
}

fn precision(min: f32, max: f32) -> usize {
  ((max - min).log10().floor() - 2.).min(-2.).abs() as usize
}

impl Compose for Slider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let u = this.modifies().subscribe(move |_| Slider::validate($this.write()));

      let mut row = @Row { align_items: Align::Center };
      let track1 = @Expanded { flex: pipe!($this.ratio())};
      let track2 = @Expanded { flex: pipe!(1. - $this.ratio())};
      @ $row {
        on_tap: move |e| {
          let width = $row.layout_size().width;
          $this.write().set_to(e.position().x / width);
        },
        on_disposed: move |_| u.unsubscribe(),
        @ {
          Provider::new(Box::new(track1.clone_writer())).with_child(
            fn_widget!{   @$track1 { @Void { class: SLIDER_ACTIVE_TRACK } } }
          )
        }
        @ Void {
          class: SLIDER_INDICATOR ,
          tooltips: pipe!($this.value).map(move |v| {
            let precision = precision($this.min, $this.max);
            format!("{:.1$}", v, precision)
          }),
        }
        @ {
          Provider::new(Box::new(track2.clone_writer())).with_child(
            fn_widget! { @ $track2 { @Void { class: SLIDER_INACTIVE_TRACK } } }
          )
        }
      }
    }
    .into_widget()
  }
}

#[derive(Declare)]
pub struct RangeSlider {
  pub start: f32,
  pub end: f32,
  #[declare(default = 100.)]
  pub max: f32,
  #[declare(default = 0.)]
  pub min: f32,
  #[declare(default)]
  pub divisions: Option<usize>,
}

impl RangeSlider {
  fn set_to(&mut self, mut ratio: f32) {
    if let Some(divisions) = self.divisions {
      ratio = (ratio * divisions as f32).round() / (divisions as f32);
    }
    let mut val = self.min + ratio * (self.max - self.min);
    val = val.clamp(self.min, self.max);

    if (self.start - val).abs() < (self.end - val).abs() {
      self.start = val;
    } else {
      self.end = val;
    }
  }

  fn left_track_ratio(&self) -> f32 {
    if self.min >= self.max {
      return 0.;
    }
    let v = (self.start - self.min) / (self.max - self.min);
    v.clamp(0., 1.)
  }

  fn middle_track_ratio(&self) -> f32 {
    if self.min >= self.max {
      return 1.;
    }

    let v = (self.end - self.start) / (self.max - self.min);
    v.clamp(0., 1.)
  }

  fn right_track_ratio(&self) -> f32 {
    if self.min >= self.max {
      return 0.;
    }

    let v = (self.max - self.end) / (self.max - self.min);
    v.clamp(0., 1.)
  }

  fn validate(mut this: WriteRef<Self>) {
    if this.max < this.min {
      let Self { max, min, .. } = &mut *this;
      swap(max, min);
    }

    if this.start > this.end {
      let Self { start, end, .. } = &mut *this;
      swap(start, end);
    }

    if this.start < this.min {
      this.start = this.min;
    }

    if this.end > this.max {
      this.end = this.max;
    }
  }
}

impl Compose for RangeSlider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let u = this.modifies().subscribe(move |_| RangeSlider::validate($this.write()));

      let mut row = @Row { align_items: Align::Center };
      let track1 = @Expanded { flex: pipe!($this.left_track_ratio()) };
      let track2 = @Expanded { flex: pipe!($this.middle_track_ratio()) };
      let track3 = @Expanded { flex: pipe!($this.right_track_ratio()) };

      @ $row {
        on_tap: move |e| {
          let width = $row.layout_size().width;
          $this.write().set_to(e.position().x / width);
        },
        on_disposed: move |_| u.unsubscribe(),

        @ {
          Provider::new(Box::new(track1.clone_writer())).with_child(
            fn_widget!{  @ $track1 { @ Void{ class: RANGE_SLIDER_INACTIVE_TRACK_LEFT } } }
          )
        }
        @Void {
          class: SLIDER_INDICATOR,
          tooltips: pipe!($this.start).map(move |v| {
            let precision = precision($this.min, $this.max);
            format!("{:.1$}", v, precision)
          }),
        }
        @ {
          Provider::new(Box::new(track2.clone_writer())).with_child(
            fn_widget! {  @ $track2 { @ Void { class: RANGE_SLIDER_ACTIVE_TRACK } } }
          )
        }
        @Void {
          class: SLIDER_INDICATOR,
          tooltips: pipe!($this.end).map(move |v| {
            let precision = precision($this.min, $this.max);
            format!("{:.1$}", v, precision)
          }),
        }
        @ {
          Provider::new(Box::new(track3.clone_writer())).with_child(
            fn_widget! { @ $track3 { @ Void { class: RANGE_SLIDER_INACTIVE_TRACK_RIGHT } } }
          )
        }
      }
    }
    .into_widget()
  }
}
