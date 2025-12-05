//! XMB Input Handling
//!
//! Processes keyboard and gamepad input for XMB navigation

use super::menu::XMBAction;
use super::state::XMBState;
use macroquad::prelude::*;

/// Result of processing input for one frame
#[derive(Debug, Clone, PartialEq)]
pub enum XMBInputResult {
    /// No action this frame
    None,
    /// User wants to activate the selected item
    Activate(XMBAction),
    /// User wants to cancel/go back
    Cancel,
}

/// Process input and update XMB state
pub fn process_input(state: &mut XMBState) -> XMBInputResult {
    // Navigation
    if is_key_pressed(KeyCode::Left) {
        state.move_left();
    }
    if is_key_pressed(KeyCode::Right) {
        state.move_right();
    }
    if is_key_pressed(KeyCode::Up) {
        state.move_up();
    }
    if is_key_pressed(KeyCode::Down) {
        state.move_down();
    }

    // Activation
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        let action = state.get_selected_action();
        return XMBInputResult::Activate(action);
    }

    // Cancel
    if is_key_pressed(KeyCode::Escape) {
        return XMBInputResult::Cancel;
    }

    XMBInputResult::None
}

/// Alternative input handler that returns bool for activation only
/// (useful for simpler integration)
pub fn check_activation(state: &mut XMBState) -> Option<XMBAction> {
    // Navigation
    if is_key_pressed(KeyCode::Left) {
        state.move_left();
    }
    if is_key_pressed(KeyCode::Right) {
        state.move_right();
    }
    if is_key_pressed(KeyCode::Up) {
        state.move_up();
    }
    if is_key_pressed(KeyCode::Down) {
        state.move_down();
    }

    // Check for activation
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        return Some(state.get_selected_action());
    }

    None
}
