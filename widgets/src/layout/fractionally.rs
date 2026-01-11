use ribir_core::{prelude::*, wrap_render::WrapRender};

/// A container that sizes its child to specified fractions of available space.
///
/// When containing a child:
/// - Constrains child width to `width_factor` × available width
/// - Constrains child height to `height_factor` × available height
///
/// When empty:
/// - Sizes itself using the same factors
/// - Defaults to 20×20 pixels if space is unconstrained or factors are invalid
#[declare]
pub struct FractionallySizedBox {
  #[declare(default = 1.0)]
  pub width_factor: f32,
  #[declare(default = 1.0)]
  pub height_factor: f32,
}

/// A container that constrains its child's width to a fraction of available
/// space.
///
/// When containing a child:
/// - Child width is `factor` × available width
/// - Child height uses available space
///
/// When empty:
/// - Width follows same factor rules
/// - Height defaults to 20 pixels (subject to layout constraints)
#[declare]
pub struct FractionallyWidthBox {
  #[declare(default = 1.0)]
  pub factor: f32,
}

/// A container that constrains its child's height to a fraction of available
/// space.
///
/// When containing a child:
/// - Child height is `factor` × available height
/// - Child width uses available space
///
/// When empty:
/// - Height follows same factor rules
/// - Width defaults to 20 pixels (subject to layout constraints)
#[declare]
pub struct FractionallyHeightBox {
  #[declare(default = 1.0)]
  pub factor: f32,
}

// ----------------------------------------------------------------------------
// FractionallySizedBox implementation
// ----------------------------------------------------------------------------

impl Render for FractionallySizedBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let clamp = fractionally_width_clamp(self.width_factor, clamp);
    let clamp = fractionally_height_clamp(self.height_factor, clamp);
    clamp.clamp(Size::splat(20.))
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallySizedBox);

impl WrapRender for FractionallySizedBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let clamp = fractionally_width_clamp(self.width_factor, clamp);
    let clamp = fractionally_height_clamp(self.height_factor, clamp);
    host.perform_layout(clamp, ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

// ----------------------------------------------------------------------------
// FractionallyWidthBox implementation
// ----------------------------------------------------------------------------

impl Render for FractionallyWidthBox {
  /// Calculates width from factor, height from default/layout constraints
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let clamp = fractionally_width_clamp(self.factor, clamp);
    clamp.clamp(Size::splat(20.))
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyWidthBox);

impl WrapRender for FractionallyWidthBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let clamp = fractionally_width_clamp(self.factor, clamp);
    host.perform_layout(clamp, ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

// ----------------------------------------------------------------------------
// FractionallyHeightBox implementation
// ----------------------------------------------------------------------------

impl Render for FractionallyHeightBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let clamp = fractionally_height_clamp(self.factor, clamp);
    clamp.clamp(Size::splat(20.))
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyHeightBox);

impl WrapRender for FractionallyHeightBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let clamp = fractionally_height_clamp(self.factor, clamp);
    host.perform_layout(clamp, ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

// ----------------------------------------------------------------------------
// Constraint calculation utilities
// ----------------------------------------------------------------------------

/// Calculates width constraint based on available space and width factor
fn fractionally_width_clamp(factor: f32, clamp: BoxClamp) -> BoxClamp {
  let max = clamp.max.width;
  if max.is_finite() {
    let factor = factor.clamp(0., 1.);
    let width = (max * factor).clamp(clamp.min.width, clamp.max.width);
    clamp.with_fixed_width(width)
  } else {
    clamp
  }
}

/// Calculates height constraint based on available space and height factor
fn fractionally_height_clamp(factor: f32, clamp: BoxClamp) -> BoxClamp {
  let max = clamp.max.height;
  if max.is_finite() {
    let factor = factor.clamp(0., 1.);
    let height = (max * factor).clamp(clamp.min.height, clamp.max.height);
    clamp.with_fixed_height(height)
  } else {
    clamp
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_layout_test! {
    fractionally_sized_box,
    WidgetTester::new(fn_widget! {
      @FractionallySizedBox {
        width_factor: 0.5,  height_factor: 0.5,
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(50., 50.))
  }

  widget_layout_test! {
    fractionally_sized_box_with_child,
    WidgetTester::new(fn_widget! {
      @FractionallySizedBox {
        width_factor: 0.,  height_factor: 1.2,
        @ { Void }
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(0., 100.))
  }

  widget_layout_test! {
    fractionally_width_box,
    WidgetTester::new(fn_widget! {
      @FractionallyWidthBox { factor: 0.5 }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(50., 20.))
  }

  widget_layout_test! {
    fractionally_width_box_with_child,
    WidgetTester::new(fn_widget! {
      @FractionallyWidthBox {
        factor: 0.5,
        @ { Void }
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(50., 0.))
  }

  widget_layout_test! {
    fractionally_height_box,
    WidgetTester::new(fn_widget! {
      @FractionallyHeightBox { factor: 0.5 }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(20., 50.))
  }

  widget_layout_test! {
    fractionally_height_box_with_child,
    WidgetTester::new(fn_widget! {
      @FractionallyHeightBox {
        factor: 0.5,
        @ { Void }
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(0., 50.))
  }
}
