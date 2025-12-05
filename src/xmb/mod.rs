//! XMB (Cross Media Bar) Menu System
//!
//! PS1-style landing page for launching different apps within Bonnie Engine.
//! Inspired by PlayStation's iconic XMB interface.

pub mod menu;
pub mod state;
pub mod render;
pub mod input;

// Re-export commonly used types
pub use menu::{XMBAction, XMBCategory, XMBItem, IconType, create_default_menu};
pub use state::XMBState;
pub use render::draw_xmb;
pub use input::{process_input, check_activation, XMBInputResult};
