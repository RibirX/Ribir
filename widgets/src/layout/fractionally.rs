use ribir_core::{prelude::*, wrap_render::WrapRender};

/// This widget resizes its child to occupy a fraction of the total available
/// space.
///
/// Alternatively, it can function as an empty box, occupying a fraction of the
/// total available space.
#[derive(Declare)]
pub struct FractionallySizedBox {
  #[declare(default = 1.0)]
  pub width_factor: f32,
  #[declare(default = 1.0)]
  pub height_factor: f32,
}

/// This widget sizes its child to occupy a fraction of the total available
/// space along the x-axis.
///
/// Alternatively, it can act as an empty box, occupying a fraction of the
/// x-axis space while extending along the y-axis.
#[derive(Declare)]
pub struct FractionallyWidthBox {
  #[declare(default = 1.0)]
  pub factor: f32,
}

/// This widget sizes its child to occupy a fraction of the total available
/// space along the y-axis.
///
/// Alternatively, it can act as an empty box, occupying a fraction of the
/// y-axis space while extending along the x-axis.
#[derive(Declare)]
pub struct FractionallyHeightBox {
  #[declare(default = 1.0)]
  pub factor: f32,
}

// implementation for FractionallySizedBox
impl Render for FractionallySizedBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let max = clamp.max;
    let mut size = clamp.min;

    if max.width.is_finite() {
      let factor = self.width_factor.clamp(0., 1.);
      let width = max.width * factor;
      size.width = width.clamp(clamp.min.width, clamp.max.width);
    }

    if max.height.is_finite() {
      let factor = self.height_factor.clamp(0., 1.);
      let height = max.height * factor;
      size.height = height.clamp(clamp.min.height, clamp.max.height);
    }

    size
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallySizedBox, DirtyPhase::Layout);

impl WrapRender for FractionallySizedBox {
  fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.max;
    if max.width.is_finite() {
      let factor = self.width_factor.clamp(0., 1.);
      let width = max.width * factor;
      clamp = clamp.with_fixed_width(width);
    }

    if max.height.is_finite() {
      let factor = self.height_factor.clamp(0., 1.);
      let height = max.height * factor;
      clamp = clamp.with_fixed_height(height);
    }

    host.perform_layout(clamp, ctx)
  }
}

// implementation for FractionallyWidthBox
impl Render for FractionallyWidthBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    if clamp.max.width.is_finite() {
      let factor = self.factor.clamp(0., 1.);
      let height = if clamp.max.height.is_finite() { clamp.max.height } else { clamp.min.height };
      Size::new(clamp.max.width * factor, height)
    } else {
      clamp.min
    }
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyWidthBox, DirtyPhase::Layout);

impl WrapRender for FractionallyWidthBox {
  fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.max.width;
    if max.is_finite() {
      let factor = self.factor.clamp(0., 1.);
      let width = (max * factor).clamp(clamp.min.width, clamp.max.width);
      clamp = clamp.with_fixed_width(width);
    }
    host.perform_layout(clamp, ctx)
  }
}

// implementation for FractionallyHeightBox

impl Render for FractionallyHeightBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    if clamp.max.height.is_finite() {
      let factor = self.factor.clamp(0., 1.);
      let width = if clamp.max.width.is_finite() { clamp.max.width } else { clamp.min.width };
      Size::new(width, clamp.max.height * factor)
    } else {
      clamp.min
    }
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyHeightBox, DirtyPhase::Layout);

impl WrapRender for FractionallyHeightBox {
  fn perform_layout(&self, mut clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.max.height;
    if max.is_finite() {
      let factor = self.factor.clamp(0., 1.);
      let height = (max * factor).clamp(clamp.min.height, clamp.max.height);
      clamp = clamp.with_fixed_height(height)
    }

    host.perform_layout(clamp, ctx)
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
    LayoutCase::default().with_size(Size::new(50., 100.))
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
    LayoutCase::default().with_size(Size::new(100., 50.))
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
