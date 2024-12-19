use crate::prelude::*;

/// The widget utilizes empty space to surround the child widget.
///
/// Both `Padding` and `Margin` utilize empty space to surround the child
/// widget. The difference lies in the fact that when used as a built-in widget,
/// padding makes the empty space appear as a part of the child widget, whereas
/// margin does not. For example:
///
/// ```
/// use ribir::prelude::*;
///
/// let _padding = text! {
///   text: "Background includes the empty space",
///   padding: EdgeInsets::all(10.),
///   background: Color::GREEN,
/// };
///
/// let _margin = text! {
///   text: "Background does not include the empty space",
///   margin: EdgeInsets::all(10.),
///   background: Color::GREEN,
/// };
/// ```
#[derive(Default, SingleChild)]
pub struct Padding {
  pub padding: EdgeInsets,
}

impl Declare for Padding {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Render for Padding {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    space_around_layout(&self.padding, &clamp, ctx)
  }
}

impl Padding {
  #[inline]
  pub fn new(padding: EdgeInsets) -> Self { Self { padding } }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @MockMulti {
        padding: EdgeInsets::only_left(1.),
        @MockBox {
           size: Size::new(100., 100.),
        }
      }
    }),
    // Padding widget
    LayoutCase::default().with_size(Size::new(101., 100.)),
    // MockMulti widget
    LayoutCase::new(&[0, 0])
      .with_size(Size::new(100., 100.))
      .with_x(1.),
    // MockBox
    LayoutCase::new(&[0, 0, 0])
      .with_size(Size::new(100., 100.))
      .with_x(0.)
  );

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn fix_padding_draw() {
    crate::reset_test_env!();

    assert_widget_eq_image!(
      WidgetTester::new(text! {
        padding: EdgeInsets::all(10.),
        background: Color::GREEN,
        text: "Hello, Ribir!"
      })
      .with_wnd_size(Size::new(128., 48.))
      .with_comparison(0.000025),
      "padding_draw"
    );
  }
}
