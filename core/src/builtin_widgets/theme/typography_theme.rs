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
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<'_, Self> {
    // At least one application theme exists
    Provider::of(ctx).unwrap()
  }

  /// Retrieve the nearest `TypographyTheme` from the context among its
  /// ancestors and return a write reference to the theme.
  pub fn write_of(ctx: &impl AsRef<ProviderCtx>) -> WriteRef<'_, Self> {
    // At least one application theme exists
    Provider::write_of(ctx).unwrap()
  }
}

/// Returns platform-specific font fallback families with comprehensive language
/// support including emoji, CJK, and international characters.
///
/// The list prioritizes system fonts first, followed by common fallbacks.
/// Fonts are ordered by likelihood of containing required glyphs.
pub fn fallback_font_families() -> &'static [&'static str] {
  #[cfg(target_os = "android")]
  const FAMILIES: &[&str] = &[
    "Roboto",              // System default
    "Noto Sans CJK SC",    // CJK optimized
    "Noto Color Emoji",    // Google emoji
    "Droid Sans Fallback", // Legacy fallback
    "Source Han Sans",     // Adobe CJK
    "Noto Serif",          // Serif variant
    "Google Sans",         // Modern UI
    "Motoya L Maru",       // Japanese rounded font
    "Padauk",              // Southeast Asian
  ];

  #[cfg(target_os = "ios")]
  const FAMILIES: &[&str] = &[
    ".AppleSystemUIFont", // System default
    "Apple Color Emoji",  // iOS emoji
    "PingFang SC",        // Chinese UI
    "Hiragino Sans",      // Japanese
    "Noto Sans CJK SC",   // CJK fallback
    "Chalkboard SE",      // iOS serif
    "Geeza Pro",          // Arabic
    "Kefa",               // African scripts
    "Roboto",             // Cross-platform
  ];

  #[cfg(target_os = "linux")]
  const FAMILIES: &[&str] = &[
    "Noto Sans",           // Google's international font
    "Noto Sans CJK SC",    // CJK support
    "DejaVu Sans",         // General Unicode
    "FreeSans",            // Linux system font
    "WenQuanYi Zen Hei",   // Chinese handwriting
    "Symbola",             // Ancient scripts/emoji
    "Noto Color Emoji",    // Google emoji
    "Roboto",              // Material design
    "Droid Sans Fallback", // Android-derived fallback
  ];

  #[cfg(target_os = "macos")]
  const FAMILIES: &[&str] = &[
    ".SF NS",            // San Francisco (system default)
    "Menlo",             // macOS monospace
    "Apple Color Emoji", // Apple emoji
    "Noto Sans CJK SC",  // Pan-CJK fallback
    "PingFang SC",       // Simplified Chinese
    "Hiragino Sans",     // Japanese
    "Geneva",            // Legacy system UI
    "Arial Unicode MS",  // Broad Unicode support
    "Roboto",            // Cross-platform fallback
    "Source Han Sans",   // Adobe's CJK font
  ];

  #[cfg(target_os = "windows")]
  const FAMILIES: &[&str] = &[
    "Segoe UI",         // System UI font
    "Segoe UI Emoji",   // Windows emoji
    "Segoe UI Symbol",  // Symbols
    "Microsoft YaHei",  // Simplified Chinese UI
    "SimSun",           // Legacy CJK
    "Nirmala UI",       // South Asian scripts
    "Noto Sans CJK SC", // CJK fallback
    "Arial",            // Western fallback
    "Roboto",           // Cross-platform UI
    "Times New Roman",  // Serif fallback
  ];

  // Default empty fallback for unsupported platforms
  #[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
  )))]
  const FAMILIES: &[&str] = &[];

  FAMILIES
}
