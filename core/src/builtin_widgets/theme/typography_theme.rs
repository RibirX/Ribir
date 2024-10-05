use super::*;

/// Use typography to present your design and content as clearly and efficiently
/// as possible.
///
/// The names of the TextTheme properties from the [Material Design
/// spec](https://m3.material.io/styles/typography/type-scale-tokens)
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTheme {
  pub display_large: TextTheme,
  pub display_medium: TextTheme,
  pub display_small: TextTheme,
  pub headline_large: TextTheme,
  pub headline_medium: TextTheme,
  pub headline_small: TextTheme,
  pub title_large: TextTheme,
  pub title_medium: TextTheme,
  pub title_small: TextTheme,
  pub label_large: TextTheme,
  pub label_medium: TextTheme,
  pub label_small: TextTheme,
  pub body_large: TextTheme,
  pub body_medium: TextTheme,
  pub body_small: TextTheme,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextTheme {
  pub text: ribir_painter::TextStyle,
  pub decoration: TextDecorationStyle,
}

/// Encapsulates the text decoration style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationStyle {
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: Brush,
}

bitflags! {
  /// A linear decoration to draw near the text.
  #[derive(Default, PartialEq, Eq, Clone, Copy, Debug)]
  pub struct  TextDecoration: u8 {
    const NONE = 0b0001;
    /// Draw a line underneath each line of text
    const UNDERLINE =  0b0010;
    /// Draw a line above each line of text
    const OVERLINE = 0b0100;
    /// Draw a line through each line of text
    const THROUGHLINE = 0b1000;
  }
}

impl TypographyTheme {
  /// Retrieve the nearest `TypographyTheme` from the context among its
  /// ancestors
  pub fn of(ctx: &impl ProviderCtx) -> QueryRef<Self> {
    // At least one application theme exists
    Provider::of(ctx).unwrap()
  }

  /// Retrieve the nearest `TypographyTheme` from the context among its
  /// ancestors and return a write reference to the theme.
  pub fn write_of(ctx: &impl ProviderCtx) -> WriteRef<Self> {
    // At least one application theme exists
    Provider::write_of(ctx).unwrap()
  }
}

impl ComposeChild<'static> for TypographyTheme {
  type Child = GenWidget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    Provider::new(Box::new(this.clone_writer()))
      .with_child(fn_widget! {
        pipe!($this;).map(move |_| child.gen_widget())
      })
      .into_widget()
  }
}
