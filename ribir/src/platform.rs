/// for platform specific code

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::register_platform_app_events_handlers;

#[cfg(not(target_os = "macos"))]
pub fn register_platform_app_events_handlers() {}
