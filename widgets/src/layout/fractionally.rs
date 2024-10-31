use ribir_core::{prelude::*, wrap_render::WrapRender};

/// This widget resizes its child to occupy a fraction of the total available
/// space. Alternatively, it can function as an empty box, occupying a fraction
/// of the total available space.
#[derive(Declare)]
pub struct FractionallySizedBox {
  pub width_factor: f32,
  pub height_factor: f32,
}

/// This widget sizes its child to occupy a fraction of the total available
/// space along the x-axis. Alternatively, it can act as an empty box, occupying
/// a fraction of the x-axis space while extending along the y-axis.
#[derive(Declare)]
pub struct FractionallyWidthBox {
  pub factor: f32,
}

/// This widget sizes its child to occupy a fraction of the total available
/// space along the y-axis. Alternatively, it can act as an empty box, occupying
/// a fraction of the y-axis space while extending along the x-axis.
#[derive(Declare)]
pub struct FractionallyHeightBox {
  pub factor: f32,
}

// implementation for FractionallySizedBox
impl Render for FractionallySizedBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { self.size(clamp) }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallySizedBox);

impl WrapRender for FractionallySizedBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let size = self.size(clamp);
    host.perform_layout(BoxClamp::fixed_size(size), ctx)
  }
}

impl FractionallySizedBox {
  fn size(&self, clamp: BoxClamp) -> Size {
    let w_factor = self.width_factor.clamp(0., 1.);
    let h_factor = self.height_factor.clamp(0., 1.);
    let size = Size::new(clamp.max.width * w_factor, clamp.max.height * h_factor);
    clamp.clamp(size)
  }
}

// implementation for FractionallyWidthBox
impl Render for FractionallyWidthBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let width = self.width(clamp);
    Size::new(width, clamp.max.height)
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyWidthBox);

impl WrapRender for FractionallyWidthBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let width = self.width(clamp);
    host.perform_layout(clamp.with_fixed_width(width), ctx)
  }
}

impl FractionallyWidthBox {
  fn width(&self, clamp: BoxClamp) -> f32 {
    let factor = self.factor.clamp(0., 1.);
    let width = clamp.max.width * factor;
    width.clamp(clamp.min.width, clamp.max.width)
  }
}
// implementation for FractionallyHeightBox

impl Render for FractionallyHeightBox {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    let height = self.height(clamp);
    Size::new(clamp.max.width, height)
  }
}

ribir_core::impl_compose_child_for_wrap_render!(FractionallyHeightBox);

impl WrapRender for FractionallyHeightBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let height = self.height(clamp);
    host.perform_layout(clamp.with_fixed_height(height), ctx)
  }
}

impl FractionallyHeightBox {
  fn height(&self, clamp: BoxClamp) -> f32 {
    let factor = self.factor.clamp(0., 1.);
    let height = clamp.max.height * factor;
    height.clamp(clamp.min.height, clamp.max.height)
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
