#![allow(clippy::needless_lifetimes)]

mod cow_rc;
mod frame_cache;
pub use cow_rc::{CowArc, Substr};
pub use frame_cache::*;
mod resource;
// Re-export rclite::Rc for convenience
pub use rclite::Rc;
pub use resource::*;
