//! XMB Input Handling
//!
//! Processes keyboard, mouse, and gamepad input for XMB navigation

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

/// Process input and update XMB state (keyboard + mouse)
/// mouse_x and mouse_y should be in XMB coordinate space (0-320, 0-240)
pub fn process_input(state: &mut XMBState) -> XMBInputResult {
    process_input_with_mouse(state, mouse_position().0, mouse_position().1)
}

/// Process input with explicit mouse coordinates (for transformed input)
pub fn process_input_with_mouse(state: &mut XMBState, mouse_x: f32, mouse_y: f32) -> XMBInputResult {
    // Keyboard Navigation
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

    // Keyboard Activation
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        let action = state.get_selected_action();
        return XMBInputResult::Activate(action);
    }

    // Cancel
    if is_key_pressed(KeyCode::Escape) {
        return XMBInputResult::Cancel;
    }

    // Mouse Input
    let result = process_mouse_input(state, mouse_x, mouse_y);
    if result != XMBInputResult::None {
        return result;
    }

    XMBInputResult::None
}

/// Process mouse input (hover selection and click activation)
fn process_mouse_input(state: &mut XMBState, mouse_x: f32, mouse_y: f32) -> XMBInputResult {
    // Use actual screen dimensions since XMB now renders directly to screen
    let screen_w = screen_width();
    let center_x = screen_w / 2.0;

    // Check category bar hover
    let category_hit = check_category_hover(state, mouse_x, mouse_y, center_x);
    if let Some(cat_idx) = category_hit {
        if cat_idx != state.selected_category {
            state.selected_category = cat_idx;
            state.selected_item = 0; // Reset item selection
        }
    }

    // Check item list hover
    let item_hit = check_item_hover(state, mouse_x, mouse_y, center_x);
    if let Some(item_idx) = item_hit {
        if item_idx != state.selected_item {
            state.selected_item = item_idx;
        }
    }

    // Check for click activation
    if is_mouse_button_pressed(MouseButton::Left) {
        if item_hit.is_some() {
            let action = state.get_selected_action();
            return XMBInputResult::Activate(action);
        }
    }

    XMBInputResult::None
}

/// Check if mouse is hovering over a category
fn check_category_hover(state: &XMBState, mouse_x: f32, mouse_y: f32, center_x: f32) -> Option<usize> {
    use super::render::layout;

    let screen_h = screen_height();
    let screen_w = screen_width();
    let y = screen_h * layout::CATEGORY_Y_PERCENT;
    let spacing = screen_w * layout::CATEGORY_SPACING_PERCENT;
    let font_size = (screen_h * layout::CATEGORY_FONT_PERCENT).max(12.0) as u16;

    for (idx, category) in state.categories.iter().enumerate() {
        let offset_from_selected = idx as f32 - state.category_scroll;
        let x = center_x + offset_from_selected * spacing;

        // Measure text bounds
        let text_dims = measure_text(&category.label, None, font_size, 1.0);
        let text_x = x - text_dims.width / 2.0;
        let text_y = y - text_dims.height;

        // Check if mouse is within bounds (with some padding)
        let padding = 10.0;
        if mouse_x >= text_x - padding
            && mouse_x <= text_x + text_dims.width + padding
            && mouse_y >= text_y - padding
            && mouse_y <= text_y + text_dims.height + padding
        {
            return Some(idx);
        }
    }

    None
}

/// Check if mouse is hovering over an item
fn check_item_hover(state: &XMBState, mouse_x: f32, mouse_y: f32, center_x: f32) -> Option<usize> {
    use super::render::layout;

    let screen_h = screen_height();
    let base_y = screen_h * layout::ITEM_LIST_Y_PERCENT;
    let spacing = screen_h * layout::ITEM_SPACING_PERCENT;
    let font_size = (screen_h * layout::ITEM_FONT_PERCENT).max(10.0) as u16;

    if let Some(category) = state.get_selected_category() {
        for (idx, item) in category.items.iter().enumerate() {
            let offset_from_selected = idx as f32 - state.item_scroll;
            let y = base_y + offset_from_selected * spacing;

            // Measure text bounds (centered like rendering)
            let text_dims = measure_text(&item.label, None, font_size, 1.0);
            let text_x = center_x - text_dims.width / 2.0;
            let text_y = y - text_dims.height;

            // Check if mouse is within bounds (with some padding)
            let padding = 15.0;
            if mouse_x >= text_x - padding
                && mouse_x <= text_x + text_dims.width + padding
                && mouse_y >= text_y - padding
                && mouse_y <= text_y + text_dims.height + padding
            {
                return Some(idx);
            }
        }
    }

    None
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
