//! World module - TR1-style room-based level system
//!
//! Clean architecture for PS1-style 3D environments:
//! - Room-based geometry with portal connectivity
//! - Visibility culling through portals
//! - Tile-based collision detection

mod geometry;
mod level;

pub use geometry::*;
pub use level::*;
