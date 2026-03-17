//! Standalone text backend crate for Ribir.
//!
//! This crate owns the shared text API surface together with the production
//! Parley-based backend implementation.

pub mod attributed_text;
pub mod font;
pub mod paint;
pub mod paragraph;
pub mod raster;
pub mod style;

mod parley_backend;
mod services;
mod svg_glyph;

pub use attributed_text::*;
pub use font::*;
pub use paint::*;
pub use paragraph::*;
pub use raster::*;
pub use services::{TextBuffer, TextServices, new_text_services};
pub use style::*;
