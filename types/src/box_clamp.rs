use crate::{INFINITY_SIZE, Size, ZERO_SIZE};

/// Boundary constraints for size-based layout.
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

impl BoxClamp {
  pub const UNLIMITED: BoxClamp = BoxClamp { min: ZERO_SIZE, max: INFINITY_SIZE };
  /// Expand horizontally to fill available width
  pub const EXPAND_X: BoxClamp =
    BoxClamp { min: Size::new(f32::INFINITY, 0.), max: Size::new(f32::INFINITY, f32::INFINITY) };
  /// Expand vertically to fill available height
  pub const EXPAND_Y: BoxClamp =
    BoxClamp { min: Size::new(0., f32::INFINITY), max: Size::new(f32::INFINITY, f32::INFINITY) };
  /// Expand both horizontally and vertically to fill available space
  pub const EXPAND_BOTH: BoxClamp = BoxClamp { min: INFINITY_SIZE, max: INFINITY_SIZE };

  /// clamp use fixed width and unfixed height
  pub const fn fixed_width(width: f32) -> Self {
    BoxClamp { min: Size::new(width, 0.), max: Size::new(width, f32::INFINITY) }
  }

  /// clamp use fixed height and unfixed width
  pub const fn fixed_height(height: f32) -> Self {
    BoxClamp { min: Size::new(0., height), max: Size::new(f32::INFINITY, height) }
  }

  /// clamp use fixed size
  pub const fn fixed_size(size: Size) -> Self { BoxClamp { min: size, max: size } }

  pub const fn min_width(width: f32) -> Self {
    let mut clamp = Self::UNLIMITED;
    clamp.min.width = width;
    clamp
  }

  pub const fn min_height(height: f32) -> Self {
    let mut clamp = Self::UNLIMITED;
    clamp.min.height = height;
    clamp
  }

  pub const fn min_size(min: Size) -> Self {
    Self { min, max: Size::new(f32::INFINITY, f32::INFINITY) }
  }

  pub const fn max_size(max: Size) -> Self { Self { min: ZERO_SIZE, max } }

  pub const fn max_height(height: f32) -> Self {
    Self { min: ZERO_SIZE, max: Size::new(f32::INFINITY, height) }
  }

  pub const fn max_width(width: f32) -> Self {
    Self { min: ZERO_SIZE, max: Size::new(width, f32::INFINITY) }
  }

  pub const fn with_min_size(mut self, size: Size) -> Self {
    self.min = Size::new(size.width.min(self.max.width), size.height.min(self.max.height));
    self
  }

  pub const fn with_max_size(mut self, size: Size) -> Self {
    self.max = Size::new(size.width.max(self.min.width), size.height.max(self.min.height));
    self
  }

  pub const fn with_fixed_height(mut self, height: f32) -> Self {
    self.min.height = height;
    self.max.height = height;
    self
  }

  pub const fn with_fixed_width(mut self, width: f32) -> Self {
    self.min.width = width;
    self.max.width = width;
    self
  }

  pub const fn with_max_width(mut self, width: f32) -> Self {
    self.max.width = width.max(self.min.width);
    self
  }

  pub const fn with_max_height(mut self, height: f32) -> Self {
    self.max.height = height.max(self.min.height);
    self
  }

  pub const fn with_min_width(mut self, width: f32) -> Self {
    self.min.width = width.min(self.max.width);
    self
  }

  pub const fn with_min_height(mut self, height: f32) -> Self {
    self.min.height = height.min(self.max.height);
    self
  }

  /// Calculates an estimated container width during child layout phases when
  /// parent width is unknown.
  pub const fn container_width(&self, child_width: f32) -> f32 {
    let min = self.min.width;
    let max = self.max.width;
    if max.is_finite() { max } else { min.max(child_width) }
  }

  /// Calculates an estimated container height during child layout phases when
  /// parent height is unknown.
  pub const fn container_height(&self, child_height: f32) -> f32 {
    let min = self.min.height;
    let max = self.max.height;
    if max.is_finite() { max } else { min.max(child_height) }
  }

  #[inline]
  pub fn clamp(self, size: Size) -> Size { size.clamp(self.min, self.max) }

  #[inline]
  pub fn expand(mut self) -> Self {
    self.max = INFINITY_SIZE;
    self
  }

  #[inline]
  pub fn loose(mut self) -> Self {
    self.min = ZERO_SIZE;
    self
  }

  pub fn free_width(mut self) -> Self {
    self.min.width = 0.0;
    self.max.width = f32::INFINITY;
    self
  }

  pub fn free_height(mut self) -> Self {
    self.min.height = 0.0;
    self.max.height = f32::INFINITY;
    self
  }
}

impl Default for BoxClamp {
  fn default() -> Self { Self::UNLIMITED }
}
