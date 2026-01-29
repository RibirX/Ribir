use crate::{prelude::*, wrap_render::*};

/// A wrapper that constrains child to fixed width and/or height based on
/// `Measure` values.
///
/// This is a built-in `FatObj` field. Setting the `width` or `height` field
/// attaches a `FixedSize` which constrains the child's dimensions.
///
/// When using `Measure::Unit`, the percentage is calculated relative to the
/// incoming clamp's max size.
///
/// # Example
///
/// Set a widget to 50% width of its parent's max width:
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     width: 50.percent(),
///     text: "50% width"
///   }
/// };
/// ```
#[derive(Clone, Default)]
pub struct FixedSize {
  pub size: DimensionSize,
}

impl FixedSize {
  /// Gets the width dimension.
  #[inline]
  pub fn width(&self) -> Dimension { self.size.width }

  /// Sets the width dimension.
  #[inline]
  pub fn set_width(&mut self, width: impl Into<Dimension>) { self.size.width = width.into(); }

  /// Gets the height dimension.
  #[inline]
  pub fn height(&self) -> Dimension { self.size.height }

  /// Sets the height dimension.
  #[inline]
  pub fn set_height(&mut self, height: impl Into<Dimension>) { self.size.height = height.into(); }
}

/// A size type with `Dimension` for width and height, supporting both pixel
/// and percentage-based dimensions.
#[derive(Clone, Copy, Default)]
pub struct DimensionSize {
  pub width: Dimension,
  pub height: Dimension,
}

impl DimensionSize {
  /// Creates a new `DimensionSize` with the given width and height dimensions.
  #[inline]
  pub fn new(width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
    Self { width: width.into(), height: height.into() }
  }

  /// Converts the dimensions to pixel values given the maximum constraints.
  #[inline]
  pub fn into_size(self, max_width: f32, max_height: f32) -> Size {
    Size::new(self.width.into_pixel(max_width), self.height.into_pixel(max_height))
  }
}

impl From<Size> for DimensionSize {
  #[inline]
  fn from(size: Size) -> Self {
    DimensionSize { width: size.width.into(), height: size.height.into() }
  }
}

impl Lerp for DimensionSize {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    DimensionSize {
      width: self.width.lerp(&to.width, factor),
      height: self.height.lerp(&to.height, factor),
    }
  }
}

#[derive(Clone, Copy, Default)]
pub enum Dimension {
  #[default]
  Auto,
  Fixed(Measure),
}

impl<T> From<T> for Dimension
where
  T: Into<Measure>,
{
  #[inline]
  fn from(v: T) -> Self { Dimension::Fixed(v.into()) }
}

impl Lerp for Dimension {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    match (self, to) {
      (Dimension::Fixed(from), Dimension::Fixed(to)) => Dimension::Fixed(from.lerp(to, factor)),
      _ => *to,
    }
  }
}

impl Dimension {
  pub fn into_pixel(self, max: f32) -> f32 {
    match self {
      Dimension::Auto => 0.,
      Dimension::Fixed(m) => m.into_pixel(max),
    }
  }
}

impl Declare for FixedSize {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(FixedSize);

impl WrapRender for FixedSize {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    let mut new_clamp = clamp;
    if let Dimension::Fixed(w) = self.size.width {
      let fixed_w = w.into_pixel(clamp.max.width);
      let constrained_w = fixed_w.clamp(clamp.min.width, clamp.max.width);
      new_clamp = new_clamp.with_fixed_width(constrained_w);
    }
    if let Dimension::Fixed(h) = self.size.height {
      let fixed_h = h.into_pixel(clamp.max.height);
      let constrained_h = fixed_h.clamp(clamp.min.height, clamp.max.height);
      new_clamp = new_clamp.with_fixed_height(constrained_h);
    }
    host.measure(new_clamp, ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    fixed_width_pixel,
    WidgetTester::new(fn_widget! {
      @FixedSize {
        width: 100.,
        @Container {}
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 500.))
  );

  widget_layout_test!(
    fixed_height_pixel,
    WidgetTester::new(fn_widget! {
      @FixedSize {
        height: 100.,
        @Container {}
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(500., 100.))
  );

  widget_layout_test!(
    fixed_width_percent,
    WidgetTester::new(fn_widget! {
      @FixedSize {
        width: Measure::Unit(0.5),
        @Container {}
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(250., 500.))
  );

  widget_layout_test!(
    fixed_both,
    WidgetTester::new(fn_widget! {
      @FixedSize {
        width: 100.px(),
        height: 50.px(),
        @Container {}
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 50.))
  );
}
