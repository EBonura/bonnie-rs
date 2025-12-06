//! Tracker UI layout and rendering

use macroquad::prelude::*;
use crate::ui::{Rect, UiContext};
use super::state::{TrackerState, TrackerView};
use super::pattern::NUM_CHANNELS;

// Colors
const BG_COLOR: Color = Color::new(0.11, 0.11, 0.13, 1.0);
const HEADER_COLOR: Color = Color::new(0.15, 0.15, 0.18, 1.0);
const ROW_EVEN: Color = Color::new(0.13, 0.13, 0.15, 1.0);
const ROW_ODD: Color = Color::new(0.11, 0.11, 0.13, 1.0);
const ROW_BEAT: Color = Color::new(0.16, 0.14, 0.12, 1.0);
const ROW_HIGHLIGHT: Color = Color::new(0.2, 0.25, 0.3, 1.0);
const CURSOR_COLOR: Color = Color::new(0.3, 0.5, 0.8, 0.8);
const PLAYBACK_ROW_COLOR: Color = Color::new(0.4, 0.2, 0.2, 0.6);
const TEXT_COLOR: Color = Color::new(0.8, 0.8, 0.85, 1.0);
const TEXT_DIM: Color = Color::new(0.4, 0.4, 0.45, 1.0);
const NOTE_COLOR: Color = Color::new(0.9, 0.85, 0.5, 1.0);
const INST_COLOR: Color = Color::new(0.5, 0.8, 0.5, 1.0);
const VOL_COLOR: Color = Color::new(0.5, 0.7, 0.9, 1.0);
const FX_COLOR: Color = Color::new(0.9, 0.5, 0.7, 1.0);

// Layout constants
const ROW_HEIGHT: f32 = 18.0;
const CHANNEL_WIDTH: f32 = 140.0;
const ROW_NUM_WIDTH: f32 = 30.0;
const NOTE_WIDTH: f32 = 36.0;
const INST_WIDTH: f32 = 24.0;
const VOL_WIDTH: f32 = 24.0;
const FX_WIDTH: f32 = 16.0;
const FXPARAM_WIDTH: f32 = 24.0;

/// Draw the tracker interface
pub fn draw_tracker(ctx: &mut UiContext, rect: Rect, state: &mut TrackerState) {
    // Background
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BG_COLOR);

    // Split into header and main area
    let header_height = 60.0;
    let header_rect = Rect::new(rect.x, rect.y, rect.w, header_height);
    let main_rect = Rect::new(rect.x, rect.y + header_height, rect.w, rect.h - header_height);

    // Draw header (transport, info)
    draw_header(ctx, header_rect, state);

    // Draw main content based on view
    match state.view {
        TrackerView::Pattern => draw_pattern_view(ctx, main_rect, state),
        TrackerView::Arrangement => draw_arrangement_view(ctx, main_rect, state),
        TrackerView::Instruments => draw_instruments_view(ctx, main_rect, state),
    }

    // Handle input
    handle_input(ctx, state);
}

/// Draw the header with transport controls and song info
fn draw_header(ctx: &mut UiContext, rect: Rect, state: &mut TrackerState) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, HEADER_COLOR);

    let mut x = rect.x + 10.0;
    let y = rect.y + 10.0;

    // View mode buttons
    let views = [
        (TrackerView::Pattern, "Pattern"),
        (TrackerView::Arrangement, "Arrange"),
        (TrackerView::Instruments, "Instr"),
    ];

    for (view, label) in views {
        let btn_w = 70.0;
        let is_active = state.view == view;
        let color = if is_active {
            Color::new(0.3, 0.4, 0.5, 1.0)
        } else {
            Color::new(0.2, 0.2, 0.25, 1.0)
        };

        draw_rectangle(x, y, btn_w, 24.0, color);
        draw_text(label, x + 8.0, y + 16.0, 14.0, TEXT_COLOR);

        if ctx.mouse.inside(&Rect::new(x, y, btn_w, 24.0)) && is_mouse_button_pressed(MouseButton::Left) {
            state.view = view;
        }

        x += btn_w + 5.0;
    }

    x += 20.0;

    // Transport controls
    let play_label = if state.playing { "Stop" } else { "Play" };
    let btn_w = 50.0;
    draw_rectangle(x, y, btn_w, 24.0, Color::new(0.2, 0.3, 0.2, 1.0));
    draw_text(play_label, x + 8.0, y + 16.0, 14.0, TEXT_COLOR);
    if ctx.mouse.inside(&Rect::new(x, y, btn_w, 24.0)) && is_mouse_button_pressed(MouseButton::Left) {
        state.toggle_playback();
    }
    x += btn_w + 10.0;

    // BPM
    draw_text(&format!("BPM: {}", state.song.bpm), x, y + 16.0, 14.0, TEXT_COLOR);
    x += 80.0;

    // Octave
    draw_text(&format!("Oct: {}", state.octave), x, y + 16.0, 14.0, TEXT_COLOR);
    x += 60.0;

    // Edit step
    draw_text(&format!("Step: {}", state.edit_step), x, y + 16.0, 14.0, TEXT_COLOR);
    x += 60.0;

    // Current instrument
    draw_text(&format!("Inst: {:02}", state.current_instrument), x, y + 16.0, 14.0, INST_COLOR);
    x += 70.0;

    // Soundfont status
    let sf_status = state.audio.soundfont_name()
        .map(|n| format!("SF: {}", n))
        .unwrap_or_else(|| "No Soundfont".to_string());
    draw_text(&sf_status, x, y + 16.0, 14.0, if state.audio.is_loaded() { TEXT_COLOR } else { TEXT_DIM });

    // Second row - position info
    let y2 = y + 28.0;
    let pattern_num = state.song.arrangement.get(state.current_pattern_idx).copied().unwrap_or(0);
    draw_text(
        &format!("Pos: {:02}/{:02}  Pat: {:02}  Row: {:03}  Ch: {}",
                 state.current_pattern_idx,
                 state.song.arrangement.len(),
                 pattern_num,
                 state.current_row,
                 state.current_channel + 1),
        rect.x + 10.0, y2 + 16.0, 14.0, TEXT_COLOR
    );

    // Status message
    if let Some(status) = state.get_status() {
        draw_text(status, rect.x + 400.0, y2 + 16.0, 14.0, Color::new(1.0, 0.8, 0.3, 1.0));
    }
}

/// Draw the pattern editor view
fn draw_pattern_view(_ctx: &mut UiContext, rect: Rect, state: &mut TrackerState) {
    // Calculate visible rows first (before borrowing pattern)
    state.visible_rows = ((rect.h - ROW_HEIGHT) / ROW_HEIGHT) as usize;

    let pattern = match state.current_pattern() {
        Some(p) => p,
        None => return,
    };

    // Channel header
    draw_rectangle(rect.x, rect.y, rect.w, ROW_HEIGHT, HEADER_COLOR);

    let mut x = rect.x + ROW_NUM_WIDTH;
    for ch in 0..NUM_CHANNELS {
        let ch_x = x;
        draw_text(&format!("Ch {}", ch + 1), ch_x + 4.0, rect.y + 14.0, 12.0, TEXT_COLOR);
        x += CHANNEL_WIDTH;

        // Channel separator
        draw_line(x - 1.0, rect.y, x - 1.0, rect.y + rect.h, 1.0, Color::new(0.25, 0.25, 0.3, 1.0));
    }

    // Draw rows
    let start_row = state.scroll_row;
    let visible_rows = state.visible_rows;
    let end_row = (start_row + visible_rows).min(pattern.length);

    for row_idx in start_row..end_row {
        let screen_row = row_idx - start_row;
        let y = rect.y + ROW_HEIGHT + screen_row as f32 * ROW_HEIGHT;

        // Row background
        let row_bg = if state.playing && row_idx == state.playback_row && state.playback_pattern_idx == state.current_pattern_idx {
            PLAYBACK_ROW_COLOR
        } else if row_idx == state.current_row {
            ROW_HIGHLIGHT
        } else if row_idx % (state.song.rows_per_beat as usize * 4) == 0 {
            ROW_BEAT
        } else if row_idx % 2 == 0 {
            ROW_EVEN
        } else {
            ROW_ODD
        };
        draw_rectangle(rect.x, y, rect.w, ROW_HEIGHT, row_bg);

        // Row number
        let row_color = if row_idx % (state.song.rows_per_beat as usize) == 0 { TEXT_COLOR } else { TEXT_DIM };
        draw_text(&format!("{:02X}", row_idx), rect.x + 4.0, y + 14.0, 12.0, row_color);

        // Draw each channel
        let mut x = rect.x + ROW_NUM_WIDTH;
        for ch in 0..NUM_CHANNELS {
            let note = &pattern.channels[ch][row_idx];

            // Cursor highlight
            if row_idx == state.current_row && ch == state.current_channel {
                let col_x = x + match state.current_column {
                    0 => 0.0,
                    1 => NOTE_WIDTH,
                    2 => NOTE_WIDTH + INST_WIDTH,
                    3 => NOTE_WIDTH + INST_WIDTH + VOL_WIDTH,
                    _ => NOTE_WIDTH + INST_WIDTH + VOL_WIDTH + FX_WIDTH,
                };
                let col_w = match state.current_column {
                    0 => NOTE_WIDTH,
                    1 => INST_WIDTH,
                    2 => VOL_WIDTH,
                    3 => FX_WIDTH,
                    _ => FXPARAM_WIDTH,
                };
                draw_rectangle(col_x, y, col_w, ROW_HEIGHT, CURSOR_COLOR);
            }

            // Note
            let note_str = note.pitch_name().unwrap_or_else(|| "---".to_string());
            let note_color = if note.pitch.is_some() { NOTE_COLOR } else { TEXT_DIM };
            draw_text(&note_str, x + 2.0, y + 14.0, 12.0, note_color);

            // Instrument
            let inst_str = note.instrument.map(|i| format!("{:02X}", i)).unwrap_or_else(|| "--".to_string());
            let inst_color = if note.instrument.is_some() { INST_COLOR } else { TEXT_DIM };
            draw_text(&inst_str, x + NOTE_WIDTH + 2.0, y + 14.0, 12.0, inst_color);

            // Volume
            let vol_str = note.volume.map(|v| format!("{:02X}", v)).unwrap_or_else(|| "--".to_string());
            let vol_color = if note.volume.is_some() { VOL_COLOR } else { TEXT_DIM };
            draw_text(&vol_str, x + NOTE_WIDTH + INST_WIDTH + 2.0, y + 14.0, 12.0, vol_color);

            // Effect
            let fx_str = note.effect.map(|e| e.to_string()).unwrap_or_else(|| "-".to_string());
            let fx_color = if note.effect.is_some() { FX_COLOR } else { TEXT_DIM };
            draw_text(&fx_str, x + NOTE_WIDTH + INST_WIDTH + VOL_WIDTH + 2.0, y + 14.0, 12.0, fx_color);

            // Effect param
            let fxp_str = note.effect_param.map(|p| format!("{:02X}", p)).unwrap_or_else(|| "--".to_string());
            draw_text(&fxp_str, x + NOTE_WIDTH + INST_WIDTH + VOL_WIDTH + FX_WIDTH + 2.0, y + 14.0, 12.0, fx_color);

            x += CHANNEL_WIDTH;
        }
    }
}

/// Draw the arrangement view (placeholder)
fn draw_arrangement_view(_ctx: &mut UiContext, rect: Rect, state: &TrackerState) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BG_COLOR);

    // Header
    draw_text("Song Arrangement", rect.x + 10.0, rect.y + 24.0, 16.0, TEXT_COLOR);

    // Draw arrangement as list
    let mut y = rect.y + 50.0;
    for (i, &pattern_idx) in state.song.arrangement.iter().enumerate() {
        let is_current = i == state.current_pattern_idx;
        let bg = if is_current { ROW_HIGHLIGHT } else if i % 2 == 0 { ROW_EVEN } else { ROW_ODD };
        draw_rectangle(rect.x + 10.0, y, 200.0, 24.0, bg);
        draw_text(
            &format!("{:02}: Pattern {:02}", i, pattern_idx),
            rect.x + 20.0, y + 16.0, 14.0,
            if is_current { NOTE_COLOR } else { TEXT_COLOR }
        );
        y += 26.0;
    }

    draw_text("(Press + to add pattern, - to remove)", rect.x + 10.0, rect.y + rect.h - 30.0, 12.0, TEXT_DIM);
}

/// Draw the instruments view (placeholder)
fn draw_instruments_view(_ctx: &mut UiContext, rect: Rect, state: &TrackerState) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BG_COLOR);

    draw_text("Instruments (GM)", rect.x + 10.0, rect.y + 24.0, 16.0, TEXT_COLOR);

    // Show GM instrument list
    let presets = state.audio.get_preset_names();
    let mut y = rect.y + 50.0;
    let mut col = 0;

    for (_, program, name) in presets.iter().take(64) {
        let x = rect.x + 10.0 + col as f32 * 250.0;
        let is_current = *program == state.current_instrument;
        let color = if is_current { NOTE_COLOR } else { TEXT_COLOR };
        draw_text(&format!("{:02}: {}", program, name), x, y, 12.0, color);
        y += 16.0;

        if y > rect.y + rect.h - 40.0 {
            y = rect.y + 50.0;
            col += 1;
            if col > 3 {
                break;
            }
        }
    }

    draw_text("Use [ ] to change instrument, 0-9 to select directly", rect.x + 10.0, rect.y + rect.h - 30.0, 12.0, TEXT_DIM);
}

/// Handle keyboard and mouse input
fn handle_input(_ctx: &mut UiContext, state: &mut TrackerState) {
    // Navigation
    if is_key_pressed(KeyCode::Up) {
        state.cursor_up();
    }
    if is_key_pressed(KeyCode::Down) {
        state.cursor_down();
    }
    if is_key_pressed(KeyCode::Left) {
        state.cursor_left();
    }
    if is_key_pressed(KeyCode::Right) {
        state.cursor_right();
    }
    if is_key_pressed(KeyCode::Tab) {
        if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            state.prev_channel();
        } else {
            state.next_channel();
        }
    }

    // Page up/down
    if is_key_pressed(KeyCode::PageUp) {
        for _ in 0..16 {
            state.cursor_up();
        }
    }
    if is_key_pressed(KeyCode::PageDown) {
        for _ in 0..16 {
            state.cursor_down();
        }
    }

    // Home/End
    if is_key_pressed(KeyCode::Home) {
        state.current_row = 0;
        state.scroll_row = 0;
    }
    if is_key_pressed(KeyCode::End) {
        if let Some(pattern) = state.current_pattern() {
            state.current_row = pattern.length - 1;
        }
    }

    // Playback
    if is_key_pressed(KeyCode::Space) {
        state.toggle_playback();
    }
    if is_key_pressed(KeyCode::Escape) {
        state.stop_playback();
    }

    // Octave
    if is_key_pressed(KeyCode::KpAdd) || (is_key_down(KeyCode::LeftShift) && is_key_pressed(KeyCode::Equal)) {
        state.octave = (state.octave + 1).min(9);
        state.set_status(&format!("Octave: {}", state.octave), 1.0);
    }
    if is_key_pressed(KeyCode::KpSubtract) || is_key_pressed(KeyCode::Minus) {
        state.octave = state.octave.saturating_sub(1);
        state.set_status(&format!("Octave: {}", state.octave), 1.0);
    }

    // Instrument selection
    if is_key_pressed(KeyCode::LeftBracket) {
        state.current_instrument = state.current_instrument.saturating_sub(1);
        state.audio.set_program(state.current_channel as i32, state.current_instrument as i32);
        state.set_status(&format!("Instrument: {:02}", state.current_instrument), 1.0);
    }
    if is_key_pressed(KeyCode::RightBracket) {
        state.current_instrument = (state.current_instrument + 1).min(127);
        state.audio.set_program(state.current_channel as i32, state.current_instrument as i32);
        state.set_status(&format!("Instrument: {:02}", state.current_instrument), 1.0);
    }

    // Edit step
    if is_key_pressed(KeyCode::F9) {
        state.edit_step = state.edit_step.saturating_sub(1);
        state.set_status(&format!("Edit step: {}", state.edit_step), 1.0);
    }
    if is_key_pressed(KeyCode::F10) {
        state.edit_step = (state.edit_step + 1).min(16);
        state.set_status(&format!("Edit step: {}", state.edit_step), 1.0);
    }

    // Delete
    if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
        state.delete_note();
    }

    // Note entry (when in edit mode and in note column)
    if state.edit_mode && state.current_column == 0 {
        // Check for note keys
        let note_keys = [
            KeyCode::Z, KeyCode::S, KeyCode::X, KeyCode::D, KeyCode::C,
            KeyCode::V, KeyCode::G, KeyCode::B, KeyCode::H, KeyCode::N,
            KeyCode::J, KeyCode::M,
            KeyCode::Q, KeyCode::Key2, KeyCode::W, KeyCode::Key3, KeyCode::E,
            KeyCode::R, KeyCode::Key5, KeyCode::T, KeyCode::Key6, KeyCode::Y,
            KeyCode::Key7, KeyCode::U,
        ];

        for key in note_keys {
            if is_key_pressed(key) {
                if let Some(pitch) = TrackerState::key_to_note(key, state.octave) {
                    state.enter_note(pitch);
                }
            }
        }

        // Note off with period or backtick
        if is_key_pressed(KeyCode::Period) || is_key_pressed(KeyCode::Apostrophe) {
            state.enter_note_off();
        }
    }
}
