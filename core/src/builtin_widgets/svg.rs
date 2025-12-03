//! This module implements the Render trait for `Svg`, allowing it to be used
//! directly as a widget.
//!
//! # Example
//!
//! ```rust
//! use ribir::prelude::*;
//!
//! fn_widget! {
//!    let svg = Svg::parse_from_bytes(
//!      br#"<svg width="100" height="100"><rect width="100" height="100" fill="red" /></svg>"#,
//!      true, true
//!    ).unwrap();
//!    @ { svg }
//! };
//! ```
use crate::prelude::*;

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.clamp(self.size()) }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    Some(Rect::from_size(ctx.box_size()?))
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let painter = ctx.painter();
    if self.size().greater_than(size).any() {
      painter.clip(Path::rect(&Rect::from_size(size)).into());
    }

    painter.draw_svg(self);
  }
}

/// A global registry for managing named SVG assets.
///
/// This module provides a centralized system for registering and retrieving SVG
/// assets by name. It's useful for applications that need to reuse SVG icons
/// throughout the UI without reloading them multiple times.
///
/// # Examples
///
/// ```rust ignore
/// use ribir::prelude::svg_registry;
///
/// // Register an SVG with a namespaced name
/// svg_registry::register("my_app::home", home_svg);
///
/// // Retrieve the SVG later
/// if let Some(svg) = svg_registry::get("my_app::home") {
///   // Use the SVG
/// }
///
/// // Get with fallback to default icon
/// let svg = svg_registry::get_or_default("my_app::settings");
/// ```
///
/// # Naming Conventions
///
/// To prevent name conflicts between different libraries or components,
/// it's recommended to use a namespace prefix with your SVG names:
/// - `my_app::icon_name` for application-specific icons
/// - `library_name::icon_name` for library-provided icons
/// - `component::icon_name` for component-specific icons
pub mod svg_registry {
  use std::sync::{LazyLock, Mutex};

  pub use super::*;

  /// Global storage for registered SVG assets.
  /// Uses a HashMap with string keys for efficient lookup.
  static SVG_REGISTRY: LazyLock<Mutex<ahash::AHashMap<&'static str, Svg>>> =
    LazyLock::new(|| Mutex::new(ahash::AHashMap::new()));

  /// Register an SVG with a specific name for later retrieval.
  ///
  /// Once registered, the SVG can be retrieved using the same `name`
  /// parameter with [`get`](get) or [`get_or_default`](get_or_default).
  ///
  /// # Arguments
  ///
  /// * `name` - A unique identifier for the SVG. Should be namespaced to avoid
  ///   conflicts
  /// * `svg` - The SVG asset to register
  ///
  /// # Example
  ///
  /// ```rust ignore
  /// 
  /// svg_registry::register("my_app::home", home_icon_svg);
  /// ```
  ///
  /// # Note
  ///
  /// To prevent conflicts, it's recommended to add a namespace prefix from
  /// your library or application to the name, such as `ribir::add`.
  pub fn register(name: &'static str, svg: Svg) { SVG_REGISTRY.lock().unwrap().insert(name, svg); }

  /// Retrieve a named SVG that was previously registered using
  /// [`register`](register).
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the SVG to retrieve
  ///
  /// # Returns
  ///
  /// * `Some(Svg)` - If the SVG was found in the registry
  /// * `None` - If the SVG was not found
  ///
  /// # Example
  ///
  /// ```rust ignore
  /// use ribir::prelude::svg_registry;
  ///
  /// if let Some(svg) = svg_registry::get("my_app::home") {
  ///   // Use the SVG
  /// }
  /// ```
  pub fn get(name: &str) -> Option<Svg> { SVG_REGISTRY.lock().unwrap().get(name).cloned() }

  /// Retrieve a named SVG with a fallback to the default SVG if not found.
  ///
  /// This is a convenience function that combines [`get`](get) with a fallback
  /// to prevent `None` values. It's useful when you want to ensure an SVG is
  /// always returned, even if the requested one doesn't exist.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the SVG to retrieve
  ///
  /// # Returns
  ///
  /// * `Svg` - The requested SVG if found, otherwise the default SVG
  ///
  /// # Example
  ///
  /// ```rust
  /// # use ribir::prelude::svg_registry;
  ///
  /// let svg = svg_registry::get_or_default("my_app::settings");
  /// // Always returns an SVG, either the requested one or the default
  /// ```
  pub fn get_or_default(name: &str) -> Svg { get(name).unwrap_or_else(default_svg) }

  /// Provides the default fallback SVG content when a requested named asset is
  /// unavailable.
  ///
  /// This function returns the built-in missing icon SVG that is used by
  /// [`get_or_default`](get_or_default) when a requested SVG is not found.
  /// The default SVG can be customized using
  /// [`set_default_svg`](set_default_svg).
  ///
  /// # Returns
  ///
  /// * `Svg` - The current default SVG
  pub fn default_svg() -> Svg {
    static DEFAULT_SVG: LazyLock<Mutex<Svg>> =
      LazyLock::new(|| Mutex::new(include_asset!("./icon_miss.svg", "svg", inherit_stroke = true)));
    DEFAULT_SVG.lock().unwrap().clone()
  }

  /// Sets a custom SVG to be used as the default fallback when a requested SVG
  /// is not found.
  ///
  /// This replaces the built-in missing icon SVG with a custom one. Useful for
  /// applications that want to use their own branding for missing assets.
  ///
  /// # Arguments
  ///
  /// * `svg` - The SVG to use as the new default
  ///
  /// # Example
  ///
  /// ```rust ignore
  /// 
  /// # use ribir::prelude::svg_registry;
  ///
  /// svg_registry::set_default_svg(my_custom_missing_icon_svg);
  /// ```
  pub fn set_default_svg(svg: Svg) {
    static DEFAULT_SVG: LazyLock<Mutex<Svg>> =
      LazyLock::new(|| Mutex::new(include_asset!("./icon_miss.svg", "svg", inherit_fill = true)));
    let mut default_svg = DEFAULT_SVG.lock().unwrap();
    *default_svg = svg;
  }

  /// Clears all registered SVGs from the registry.
  ///
  /// This function removes all SVGs that have been registered using
  /// [`register`](register). The default SVG is unaffected by this operation.
  ///
  /// # Note
  ///
  /// Use with caution as this will remove all SVGs from the registry.
  /// Any subsequent calls to [`get`](get) will return `None` until new SVGs are
  /// registered.
  pub fn clear() { SVG_REGISTRY.lock().unwrap().clear(); }

  /// Checks if a specific SVG name is registered in the registry.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to check for existence
  ///
  /// # Returns
  ///
  /// * `bool` - `true` if the SVG is registered, `false` otherwise
  ///
  /// # Example
  ///
  /// ```rust ignore
  /// use ribir::prelude::svg_registry;
  /// if svg_registry::contains("my_app::home") {
  ///   // The SVG is available
  /// }
  /// ```
  pub fn contains(name: &str) -> bool { SVG_REGISTRY.lock().unwrap().contains_key(name) }

  /// Returns the number of SVGs currently registered in the registry.
  ///
  /// # Returns
  ///
  /// * `usize` - The count of registered SVGs
  pub fn len() -> usize { SVG_REGISTRY.lock().unwrap().len() }

  /// Returns `true` if no SVGs are currently registered in the registry.
  ///
  /// # Returns
  ///
  /// * `bool` - `true` if the registry is empty, `false` otherwise
  pub fn is_empty() -> bool { SVG_REGISTRY.lock().unwrap().is_empty() }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;

  fn svgs_smoke() -> Painter {
    svg_registry::register(
      "test::add",
      Svg::parse_from_bytes(r#"<svg xmlns="http://www.w3.org/2000/svg" height="48" width="48"><path d="M22.5 38V25.5H10v-3h12.5V10h3v12.5H38v3H25.5V38Z"/></svg>"#.as_bytes(),
      true, false
      ).unwrap(),
    );
    let mut painter = Painter::new(Rect::from_size(Size::new(128., 64.)));
    let add = svg_registry::get("test::add").unwrap();
    let x = svg_registry::get_or_default("x");

    painter
      .draw_svg(&add)
      .translate(64., 0.)
      .draw_svg(&x);

    painter
  }

  painter_backend_eq_image_test!(svgs_smoke, comparison = 0.001);
}
