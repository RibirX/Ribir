use crate::prelude::*;

/// A widget that represents an empty node in the widget tree.
///
/// This widget is used when you need a placeholder widget that doesn't render
/// anything and doesn't accept children. It's useful for conditional rendering
/// or as a neutral widget in compositions.
///
/// The `hint_size` field allows the void to express a preferred size while
/// still being constrained by external clamp constraints.
///
/// # Example
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Void {}
/// };
/// ```
#[derive(Declare, Clone, Default)]
#[declare(eager)]
pub struct Void {
  #[declare(default)]
  pub hint_size: DimensionSize,
}

impl VoidDeclarer {
  pub fn with_hint_width<K: ?Sized>(
    &mut self, hint_width: impl RInto<PipeValue<Dimension>, K>,
  ) -> &mut Self {
    let host = self.host().clone_writer();
    let mix = self.mix_builtin_widget();
    mix.init_sub_widget(hint_width, &host, |w: &mut Void, v| w.hint_size.width = v);
    self
  }

  pub fn with_hint_height<K: ?Sized>(
    &mut self, hint_height: impl RInto<PipeValue<Dimension>, K>,
  ) -> &mut Self {
    let host = self.host().clone_writer();
    let mix = self.mix_builtin_widget();
    mix.init_sub_widget(hint_height, &host, |w: &mut Void, v| w.hint_size.height = v);
    self
  }
}

impl Render for Void {
  fn measure(&self, clamp: BoxClamp, _: &mut MeasureCtx) -> Size {
    let width = match self.hint_size.width {
      Dimension::Fixed(m) => m.into_pixel(clamp.max.width),
      Dimension::Auto => clamp.min.width,
    };
    let height = match self.hint_size.height {
      Dimension::Fixed(m) => m.into_pixel(clamp.max.height),
      Dimension::Auto => clamp.min.height,
    };
    clamp.clamp(Size::new(width, height))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("void") }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    void_default,
    WidgetTester::new(fn_widget! {
      @Void {}
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(ZERO_SIZE)
  );

  widget_layout_test!(
    void_with_hint_size,
    WidgetTester::new(fn_widget! {
      @Void {
        hint_size: DimensionSize::new(100., 100.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    void_with_percent_hint,
    WidgetTester::new(fn_widget! {
      @Void {
        hint_size: DimensionSize::new(50.percent(), 50.percent()),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(250., 250.))
  );

  widget_layout_test!(
    void_hint_clamped_by_max,
    WidgetTester::new(fn_widget! {
      @Void {
        clamp: BoxClamp::max_size(Size::new(100., 100.)),
        hint_size: DimensionSize::new(200., 200.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    void_hint_clamped_by_min,
    WidgetTester::new(fn_widget! {
      @Void {
        clamp: BoxClamp::min_size(Size::new(100., 100.)),
        hint_size: DimensionSize::new(50., 50.),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    void_with_hint_width_height,
    WidgetTester::new(fn_widget! {
      @Void {
        hint_width: 100.,
        hint_height: 100.,
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    void_with_percent_hint_width_height,
    WidgetTester::new(fn_widget! {
      @Void {
        hint_width: 50.percent(),
        hint_height: 50.percent(),
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(250., 250.))
  );
}
