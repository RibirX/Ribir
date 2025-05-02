use std::{borrow::Cow, error::Error};

pub use url::Url;

use crate::prelude::*;
/// Tracks and coordinates navigation state and URL updates consistently.
///
/// Provides a single reliable source for managing where users are in the window
/// while keeping URLs in sync, whether running in browsers or native apps.
///
/// Manages window location as a reactive resource while maintaining strict
/// separation from rendering concerns. Serves as the foundation for routing
/// systems by providing consistent URL access without triggering visual updates
/// or browser page reloads.
///
/// # Key Characteristics
///
/// - **Non-visual**: Never triggers widget construction, layout, or painting
/// - **History API integration**: Preserves web app behavior without full page
///   reloads
/// - **Cross-platform**: Unified API surface for web and native widgets
/// - **Reactive**: Changes propagate through the framework's reactivity system
///
/// # Initialization
///
/// Initialize the location resource by calling [`Location::init`] in your root
/// build widget. On web platforms, the initial URL must comply with same-origin
/// policy restrictions.
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// App::run(fn_widget! {
///     Location::init("https://example.com/dashboard", BuildCtx::get()).unwrap();
///     @Text { text: "Dashboard" }
/// });
/// ```
///
/// # Usage Examples
///
/// ```rust ignore
/// // Access location through framework context
/// let location = Location::of(ctx);
///
/// // Read URL components via Deref<Url>
/// println!("Current path: {}", location.path());
///
/// // Navigate using full URL control
/// location.goto("/users/42?details=full", ctx);
/// ```
///
/// # Platform-specific Behavior
///
/// - **Web**:
///   - Integrates with [`history.pushState`](https://developer.mozilla.org/en-US/docs/Web/API/History/pushState)
///   - Handles [`popstate`](https://developer.mozilla.org/en-US/docs/Web/API/Window/popstate_event)
///     events for back/forward navigation
///   - Synchronizes hash fragments and query parameters with `window.location`
/// - **Non-web**:
///   - Maintains URL state in memory
///   - No browser history integration
///
/// # Core Operations
///
/// - Access current URL via [`Deref<Target = Url>`](std::ops::Deref)
///   implementation for direct URL component access
/// - Navigate programmatically using [`goto`](Location::goto) (pushes history
///   entry on web)
/// - Observe changes through the framework's reactivity system
pub struct Location {
  url: Url,

  #[cfg(target_arch = "wasm32")]
  /// Web: Manages popstate listener lifecycle for history integration.
  /// Maintains subscription until Location instance drops.
  closure: Option<LocationClosure>,
}

#[cfg(target_arch = "wasm32")]
type LocationClosure = web_sys::wasm_bindgen::prelude::Closure<dyn FnMut()>;

impl Location {
  /// Initializes the Location provider with a specified URL.
  pub fn init(url: &str, ctx: &impl AsRef<ProviderCtx>) -> Result<(), Box<dyn Error>> {
    let url = Url::parse(url).map_err(|e| format!("Invalid URL '{}': {}", url, e))?;

    let mut loc = Self::write_of(ctx);
    #[cfg(target_arch = "wasm32")]
    {
      #[cfg(target_arch = "wasm32")]
      loc.same_origin_check(&url)?;
      Self::update_browser_history(url.as_str())?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if loc.url != Self::fallback_url() {
      return Err(format!("The location is already initialized to '{}'", loc.url).into());
    }

    loc.url = url;
    Ok(())
  }

  /// Gets a read-only reference to the Location provider from the context
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<Self> {
    Provider::of(ctx).expect("Location provider not found")
  }

  /// Gets a stateful reference to the Location provider from the context
  pub fn state_of(ctx: &impl AsRef<ProviderCtx>) -> Stateful<Self> {
    Provider::state_of::<Stateful<Self>>(ctx)
      .expect("Location provider not found")
      .clone_writer()
  }

  /// Get a query parameter by name
  pub fn get_query(&self, name: &str) -> Option<Cow<str>> {
    self
      .url
      .query_pairs()
      .find_map(|(key, value)| (key == name).then_some(value))
  }

  /// Navigates using a relative path without page reload (web only).
  ///
  /// # Key Features
  /// - **Relative Path Only**: Explicitly rejects absolute URLs
  /// - **Base Resolution**: Automatically combines with current origin
  ///
  /// # Usage
  ///
  /// ```ignore
  /// // Valid relative path
  /// Location::goto("/about", ctx)?;
  ///
  /// // Invalid absolute URL
  /// Location::goto("https://example.com", ctx)?; // Error: absolute URL
  /// ```
  ///
  /// # Arguments
  /// - `url`: Relative path for navigation (e.g., "about" or "/contact")
  /// - `ctx`: Location state provider context
  pub fn goto(url: &str, ctx: &impl AsRef<ProviderCtx>) -> Result<(), Box<dyn Error>> {
    Self::write_of(ctx).resolve_relative(url)
  }

  /// A low-level method to apply a relative URL to the current location.
  ///
  /// Use [`goto`](Location::goto) instead of this method when possible.
  pub fn resolve_relative(&mut self, url: &str) -> Result<(), Box<dyn Error>> {
    let new_url = self
      .join(url)
      .map_err(|e| format!("Invalid path '{}': {}", url, e))?;

    #[cfg(target_arch = "wasm32")]
    Self::update_browser_history(new_url.as_str())?;

    self.url = new_url;
    Ok(())
  }

  /// Creates stateful Location instance for the window.
  #[cfg(not(target_arch = "wasm32"))]
  pub(crate) fn stateful() -> Stateful<Location> {
    Stateful::new(Location { url: Self::fallback_url() })
  }

  /// Creates stateful Location instance for the window.
  #[cfg(target_arch = "wasm32")]
  pub(crate) fn stateful() -> Stateful<Location> {
    use web_sys::wasm_bindgen::JsCast;

    let initial_url = Self::current_browser_url().unwrap_or_else(Self::fallback_url);
    let location = Stateful::new(Location { url: initial_url, closure: None });
    let popstate_handler = Self::create_popstate_handler(location.clone_writer());

    web_sys::window()
      .expect("Browser window unavailable")
      .add_event_listener_with_callback("popstate", popstate_handler.as_ref().unchecked_ref())
      .expect("Failed to register popstate listener");

    location.write().closure = Some(popstate_handler);
    location
  }

  /// Cleans up associated resources and event listeners.
  ///
  /// # Platform-Specific Behavior
  /// - **WebAssembly**: Removes registered `popstate` event listeners from the
  ///   window
  /// - **Other targets**: No-operation
  pub(crate) fn release(&mut self) {
    #[cfg(target_arch = "wasm32")]
    {
      use web_sys::wasm_bindgen::JsCast;
      if let Some(closure) = self.closure.take() {
        let _ = web_sys::window().and_then(|w| {
          w.remove_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref())
            .ok()
        });
      }
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn same_origin_check(&self, url: &Url) -> Result<(), String> {
    let self_origin = self.url.origin();
    let other_origin = url.origin();

    if self_origin == other_origin {
      return Ok(());
    }

    let self_display = self_origin.unicode_serialization();
    let other_display = other_origin.unicode_serialization();

    Err(format!("Cross-origin navigation blocked: {self_display} → {other_display}"))
  }

  fn write_of(ctx: &impl AsRef<ProviderCtx>) -> WriteRef<Location> {
    Provider::write_of(ctx).expect("Location write provider not found")
  }

  #[cfg(target_arch = "wasm32")]
  fn update_browser_history(url: &str) -> Result<(), Box<dyn Error>> {
    let history = web_sys::window()
      .and_then(|w| w.history().ok())
      .ok_or("Browser history unavailable")?;

    if Self::current_browser_url().is_some_and(|u| u.as_str() != url) {
      history
        .push_state_with_url(&web_sys::wasm_bindgen::JsValue::NULL, "", Some(url))
        .map_err(|e| format!("History API error: {:?}", e))?;
    }
    Ok(())
  }

  #[cfg(target_arch = "wasm32")]
  fn create_popstate_handler(location: Stateful<Location>) -> LocationClosure {
    web_sys::wasm_bindgen::prelude::Closure::<dyn FnMut()>::new(move || {
      if let Some(new_url) = Self::current_browser_url() {
        let mut loc = location.write();
        if loc.url != new_url {
          loc.url = new_url;
        }
      }
    })
  }

  #[cfg(target_arch = "wasm32")]
  fn current_browser_url() -> Option<Url> {
    web_sys::window()
      .and_then(|w| w.location().href().ok())
      .and_then(|href| Url::parse(&href).ok())
  }

  fn fallback_url() -> Url { Url::parse("https://ribir.org").unwrap() }
}

impl std::ops::Deref for Location {
  type Target = Url;
  fn deref(&self) -> &Self::Target { &self.url }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn goto() {
    reset_test_env!();

    let location = Location::stateful();
    let mut w = location.write();
    w.resolve_relative("a/b/c").unwrap();
    assert_eq!(w.url.as_str(), "https://ribir.org/a/b/c");

    w.resolve_relative("/a/b/d").unwrap();
    assert_eq!(w.url.as_str(), "https://ribir.org/a/b/d");

    w.resolve_relative("../e").unwrap();
    assert_eq!(w.url.as_str(), "https://ribir.org/a/e");

    w.resolve_relative("./f").unwrap();
    assert_eq!(w.url.as_str(), "https://ribir.org/a/f");
  }
}
