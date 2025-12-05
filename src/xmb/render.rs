//! XMB Rendering
//!
//! Renders the XMB menu with PS1-style aesthetics using macroquad

use super::state::XMBState;
use macroquad::prelude::*;

/// XMB visual theme colors
pub mod theme {
    use macroquad::prelude::Color;

    /// Background gradient top color (dark blue)
    pub const BG_TOP: Color = Color::new(0.04, 0.04, 0.18, 1.0);
    /// Background gradient bottom color (black)
    pub const BG_BOTTOM: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    /// Selected item color (cyan)
    pub const SELECTED: Color = Color::new(0.0, 0.83, 1.0, 1.0);
    /// Unselected item color (gray)
    pub const UNSELECTED: Color = Color::new(0.38, 0.38, 0.5, 1.0);
    /// Category color (lighter gray)
    pub const CATEGORY: Color = Color::new(0.6, 0.6, 0.7, 1.0);
    /// Description text color
    pub const DESCRIPTION: Color = Color::new(0.7, 0.7, 0.8, 1.0);
    /// Wave color (dark cyan)
    pub const WAVE: Color = Color::new(0.0, 0.2, 0.3, 0.3);
}

/// Layout constants
pub mod layout {
    /// Category bar Y position (from top)
    pub const CATEGORY_Y: f32 = 80.0;
    /// Category spacing (horizontal)
    pub const CATEGORY_SPACING: f32 = 180.0;
    /// Item list starting Y position
    pub const ITEM_LIST_Y: f32 = 180.0;
    /// Item spacing (vertical)
    pub const ITEM_SPACING: f32 = 40.0;
    /// Description Y position (from bottom)
    pub const DESCRIPTION_Y_OFFSET: f32 = 80.0;
    /// Category font size
    pub const CATEGORY_FONT_SIZE: f32 = 24.0;
    /// Item font size
    pub const ITEM_FONT_SIZE: f32 = 20.0;
    /// Description font size
    pub const DESCRIPTION_FONT_SIZE: f32 = 16.0;
}

/// Draw the XMB menu
pub fn draw_xmb(state: &XMBState) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // 1. Draw background gradient
    draw_gradient_background(screen_w, screen_h);

    // 2. Draw animated wave effect
    draw_wave_background(state.time, screen_w, screen_h);

    // 3. Draw category bar (horizontal)
    draw_category_bar(state, screen_w, screen_h);

    // 4. Draw item list (vertical)
    draw_item_list(state, screen_w, screen_h);

    // 5. Draw description at bottom
    draw_description(state, screen_w, screen_h);
}

/// Draw vertical gradient background
fn draw_gradient_background(screen_w: f32, screen_h: f32) {
    // Split screen into horizontal strips for gradient effect
    let strips = 10;
    let strip_height = screen_h / strips as f32;

    for i in 0..strips {
        let t = i as f32 / strips as f32;
        let color = Color::new(
            theme::BG_TOP.r * (1.0 - t) + theme::BG_BOTTOM.r * t,
            theme::BG_TOP.g * (1.0 - t) + theme::BG_BOTTOM.g * t,
            theme::BG_TOP.b * (1.0 - t) + theme::BG_BOTTOM.b * t,
            1.0,
        );

        let y = i as f32 * strip_height;
        draw_rectangle(0.0, y, screen_w, strip_height, color);
    }
}

/// Draw animated sine wave background (PS1-style)
fn draw_wave_background(time: f32, screen_w: f32, screen_h: f32) {
    let wave_count = 8;
    let segment_count = 40;

    for wave_idx in 0..wave_count {
        let wave_y_base = (wave_idx as f32 / wave_count as f32) * screen_h;
        let phase_offset = wave_idx as f32 * 0.5;

        for i in 0..segment_count {
            let t1 = i as f32 / segment_count as f32;
            let t2 = (i + 1) as f32 / segment_count as f32;

            let x1 = t1 * screen_w;
            let x2 = t2 * screen_w;

            // Sine wave offset
            let y1_offset = ((t1 * 8.0 + time * 2.0 + phase_offset).sin() * 20.0).floor();
            let y2_offset = ((t2 * 8.0 + time * 2.0 + phase_offset).sin() * 20.0).floor();

            let y1 = wave_y_base + y1_offset;
            let y2 = wave_y_base + y2_offset;

            draw_line(x1, y1, x2, y2, 2.0, theme::WAVE);
        }
    }
}

/// Draw the horizontal category bar
fn draw_category_bar(state: &XMBState, screen_w: f32, _screen_h: f32) {
    let center_x = screen_w / 2.0;

    for (idx, category) in state.categories.iter().enumerate() {
        let offset_from_selected = idx as f32 - state.category_scroll;
        let x = center_x + offset_from_selected * layout::CATEGORY_SPACING;
        let y = layout::CATEGORY_Y;

        // Calculate alpha based on distance from center
        let distance = offset_from_selected.abs();
        let alpha = (1.0 - (distance * 0.3).min(1.0)).max(0.0);

        // Scale based on selection
        let scale = if idx == state.selected_category {
            1.0 + state.pulse * 0.1 // Pulsing effect on selected
        } else {
            0.9 - distance * 0.05
        };

        let color = if idx == state.selected_category {
            Color::new(theme::SELECTED.r, theme::SELECTED.g, theme::SELECTED.b, alpha)
        } else {
            Color::new(theme::CATEGORY.r, theme::CATEGORY.g, theme::CATEGORY.b, alpha * 0.7)
        };

        // Center the text
        let text_dims = measure_text(&category.label, None, layout::CATEGORY_FONT_SIZE as u16, scale);
        let text_x = x - text_dims.width / 2.0;
        let text_y = y;

        draw_text_ex(
            &category.label,
            text_x,
            text_y,
            TextParams {
                font_size: layout::CATEGORY_FONT_SIZE as u16,
                font_scale: scale,
                color,
                ..Default::default()
            },
        );

        // Draw selection indicator (horizontal line under selected category)
        if idx == state.selected_category {
            let line_width = text_dims.width * 1.2;
            let line_x = x - line_width / 2.0;
            let line_y = y + 10.0;
            draw_line(
                line_x,
                line_y,
                line_x + line_width,
                line_y,
                2.0,
                Color::new(theme::SELECTED.r, theme::SELECTED.g, theme::SELECTED.b, alpha),
            );
        }
    }
}

/// Draw the vertical item list
fn draw_item_list(state: &XMBState, screen_w: f32, _screen_h: f32) {
    if let Some(category) = state.get_selected_category() {
        let center_x = screen_w / 2.0;

        for (idx, item) in category.items.iter().enumerate() {
            let offset_from_selected = idx as f32 - state.item_scroll;
            let x = center_x - 100.0; // Offset to left
            let y = layout::ITEM_LIST_Y + offset_from_selected * layout::ITEM_SPACING;

            // Calculate alpha based on distance from center
            let distance = offset_from_selected.abs();
            let alpha = (1.0 - (distance * 0.4).min(1.0)).max(0.0);

            // Scale based on selection
            let scale = if idx == state.selected_item {
                1.0 + state.pulse * 0.05
            } else {
                0.9
            };

            let color = if idx == state.selected_item {
                Color::new(theme::SELECTED.r, theme::SELECTED.g, theme::SELECTED.b, alpha)
            } else {
                Color::new(theme::UNSELECTED.r, theme::UNSELECTED.g, theme::UNSELECTED.b, alpha * 0.8)
            };

            // Draw selection arrow
            if idx == state.selected_item {
                let arrow = "▶";
                draw_text_ex(
                    arrow,
                    x - 30.0,
                    y,
                    TextParams {
                        font_size: layout::ITEM_FONT_SIZE as u16,
                        color,
                        ..Default::default()
                    },
                );
            }

            draw_text_ex(
                &item.label,
                x,
                y,
                TextParams {
                    font_size: layout::ITEM_FONT_SIZE as u16,
                    font_scale: scale,
                    color,
                    ..Default::default()
                },
            );
        }
    }
}

/// Draw description text at bottom
fn draw_description(state: &XMBState, screen_w: f32, screen_h: f32) {
    if let Some(description) = state.get_selected_description() {
        let x = screen_w / 2.0;
        let y = screen_h - layout::DESCRIPTION_Y_OFFSET;

        // Center the description text
        let text_dims = measure_text(description, None, layout::DESCRIPTION_FONT_SIZE as u16, 1.0);
        let text_x = x - text_dims.width / 2.0;

        draw_text_ex(
            description,
            text_x,
            y,
            TextParams {
                font_size: layout::DESCRIPTION_FONT_SIZE as u16,
                color: theme::DESCRIPTION,
                ..Default::default()
            },
        );
    }
}
