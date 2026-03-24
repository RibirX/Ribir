#![allow(clippy::needless_lifetimes)]

mod cow_rc;
mod frame_cache;
pub use cow_rc::{CowArc, Substr};
pub use frame_cache::*;
mod resource;
// Re-export rclite::Rc and Arc for convenience
pub use rclite::{Arc, Rc};
pub use resource::*;
