use crate::prelude::*;

/// A container widget that sizes itself based on its `hint_size` field or
/// the maximum allowed by its clamp constraints.
///
/// Container's size is determined by `hint_size` when set, otherwise it
/// defaults to `clamp.max`. This makes it ideal for creating fixed-size boxes
/// when combined with the `size` or `height`/`width` builtin attribute.
///
/// # Example
///
/// Place text inside a 100x100 container.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   background: Color::BLUE,
///   @Text { text: "Hello" }
/// };
/// ```
#[derive(Declare, SingleChild, Clone, Default)]
#[declare(eager)]
pub struct Container {
  #[declare(default)]
  pub hint_size: DimensionSize,
}

impl ContainerDeclarer {
  pub fn with_hint_width<K: ?Sized>(
    &mut self, hint_width: impl RInto<PipeValue<Dimension>, K>,
  ) -> &mut Self {
    let host = self.host().clone_writer();
    let mix = self.mix_builtin_widget();
    mix.init_sub_widget(hint_width, &host, |w: &mut Container, v| w.hint_size.width = v);
    self
  }

  pub fn with_hint_height<K: ?Sized>(
    &mut self, hint_height: impl RInto<PipeValue<Dimension>, K>,
  ) -> &mut Self {
    let host = self.host().clone_writer();
    let mix = self.mix_builtin_widget();
    mix.init_sub_widget(hint_height, &host, |w: &mut Container, v| w.hint_size.height = v);
    self
  }
}

impl Render for Container {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let width = match self.hint_size.width {
      Dimension::Fixed(m) => m.into_pixel(clamp.max.width),
      Dimension::Auto => clamp.max.width,
    };
    let height = match self.hint_size.height {
      Dimension::Fixed(m) => m.into_pixel(clamp.max.height),
      Dimension::Auto => clamp.max.height,
    };
    let size = clamp.clamp(Size::new(width, height));
    let child_clamp = BoxClamp::max_size(size);

    ctx.perform_single_child_layout(child_clamp);
    size
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { false }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("container") }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  const TEST_SIZE: Size = Size::new(100., 100.);

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @Container { size: Size::new(100., 100.) }
    }),
    LayoutCase::default().with_size(TEST_SIZE)
  );

  widget_layout_test!(
    container_with_clamp,
    WidgetTester::new(fn_widget! {
      @Container {
        clamp: BoxClamp::fixed_size(Size::new(200., 150.)),
      }
    }),
    LayoutCase::default().with_size(Size::new(200., 150.))
  );

  widget_layout_test!(
    container_with_percent_width,
    WidgetTester::new(fn_widget! {
      @Container {
        clamp: BoxClamp::max_width(400.),
        width: 50.percent(),
        height: 100.,
      }
    })
    .with_wnd_size(Size::new(800., 600.)),
    LayoutCase::default().with_size(Size::new(200., 100.))
  );

  widget_layout_test!(
    container_with_hint_size,
    WidgetTester::new(fn_widget! {
      @Container {
        hint_size: DimensionSize::new(100., 100.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    container_with_percent_hint,
    WidgetTester::new(fn_widget! {
      @Container {
        hint_size: DimensionSize::new(50.percent(), 50.percent()),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(250., 250.))
  );

  widget_layout_test!(
    container_hint_clamped_by_max,
    WidgetTester::new(fn_widget! {
      @Container {
        clamp: BoxClamp::max_size(Size::new(100., 100.)),
        hint_size: DimensionSize::new(200., 200.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    container_hint_clamped_by_min,
    WidgetTester::new(fn_widget! {
      @Container {
        clamp: BoxClamp::min_size(Size::new(100., 100.)),
        hint_size: DimensionSize::new(50., 50.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    container_with_hint_width_height,
    WidgetTester::new(fn_widget! {
      @Container {
        hint_width: 100.,
        hint_height: 100.,
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    container_with_percent_hint_width_height,
    WidgetTester::new(fn_widget! {
      @Container {
        hint_width: 50.percent(),
        hint_height: 50.percent(),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(250., 250.))
  );
}
