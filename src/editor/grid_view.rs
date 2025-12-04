//! 2D Grid View - Top-down room editing

use macroquad::prelude::*;
use crate::ui::{Rect, UiContext};
use super::{EditorState, Selection};

/// Draw the 2D grid view (top-down view of current room)
pub fn draw_grid_view(ctx: &mut UiContext, rect: Rect, state: &mut EditorState) {
    // Background
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(20, 20, 25, 255));

    let mouse_pos = (ctx.mouse.x, ctx.mouse.y);
    let inside = ctx.mouse.inside(&rect);

    // Handle pan and zoom
    if inside {
        // Zoom with scroll wheel
        if ctx.mouse.scroll != 0.0 {
            let zoom_factor = 1.0 + ctx.mouse.scroll * 0.02;
            state.grid_zoom = (state.grid_zoom * zoom_factor).clamp(5.0, 100.0);
        }

        // Pan with right mouse button
        if ctx.mouse.right_down {
            if state.grid_panning {
                let dx = mouse_pos.0 - state.grid_last_mouse.0;
                let dy = mouse_pos.1 - state.grid_last_mouse.1;
                state.grid_offset_x += dx;
                state.grid_offset_y += dy;
            }
            state.grid_panning = true;
        } else {
            state.grid_panning = false;
        }
    } else {
        state.grid_panning = false;
    }
    state.grid_last_mouse = mouse_pos;

    let Some(room) = state.current_room() else {
        draw_text("No room", rect.x + 10.0, rect.y + 20.0, 14.0, Color::from_rgba(100, 100, 100, 255));
        return;
    };

    // Calculate view transform
    let center_x = rect.x + rect.w * 0.5 + state.grid_offset_x;
    let center_y = rect.y + rect.h * 0.5 + state.grid_offset_y;
    let scale = state.grid_zoom;

    // World to screen conversion (X-Z plane, Y is up)
    let world_to_screen = |wx: f32, wz: f32| -> (f32, f32) {
        let sx = center_x + wx * scale;
        let sy = center_y + wz * scale; // Z maps to screen Y
        (sx, sy)
    };

    // Draw grid lines
    if state.show_grid {
        let grid_color = Color::from_rgba(40, 40, 45, 255);
        let grid_step = state.grid_size;

        // Calculate visible grid range
        let min_wx = (rect.x - center_x) / scale;
        let max_wx = (rect.right() - center_x) / scale;
        let min_wz = (rect.y - center_y) / scale;
        let max_wz = (rect.bottom() - center_y) / scale;

        // Vertical lines
        let start_x = (min_wx / grid_step).floor() * grid_step;
        let mut x = start_x;
        while x <= max_wx {
            let (sx, _) = world_to_screen(x, 0.0);
            if sx >= rect.x && sx <= rect.right() {
                let line_color = if (x / grid_step).abs() < 0.01 {
                    Color::from_rgba(80, 40, 40, 255) // Origin line (red-ish)
                } else {
                    grid_color
                };
                draw_line(sx, rect.y, sx, rect.bottom(), 1.0, line_color);
            }
            x += grid_step;
        }

        // Horizontal lines
        let start_z = (min_wz / grid_step).floor() * grid_step;
        let mut z = start_z;
        while z <= max_wz {
            let (_, sy) = world_to_screen(0.0, z);
            if sy >= rect.y && sy <= rect.bottom() {
                let line_color = if (z / grid_step).abs() < 0.01 {
                    Color::from_rgba(40, 80, 40, 255) // Origin line (green-ish)
                } else {
                    grid_color
                };
                draw_line(rect.x, sy, rect.right(), sy, 1.0, line_color);
            }
            z += grid_step;
        }
    }

    // Draw room geometry (X-Z projection)
    // First pass: draw faces as filled polygons
    for (face_idx, face) in room.faces.iter().enumerate() {
        let v0 = room.vertices[face.indices[0]];
        let v1 = room.vertices[face.indices[1]];
        let v2 = room.vertices[face.indices[2]];
        let v3 = room.vertices[face.indices[3]];

        let (sx0, sy0) = world_to_screen(v0.x, v0.z);
        let (sx1, sy1) = world_to_screen(v1.x, v1.z);
        let (sx2, sy2) = world_to_screen(v2.x, v2.z);
        let (sx3, sy3) = world_to_screen(v3.x, v3.z);

        // Determine face type by normal (approximate from Y component)
        // Floor faces have normal pointing up (negative Y in our system)
        let edge1 = (v1.x - v0.x, v1.y - v0.y, v1.z - v0.z);
        let edge2 = (v2.x - v0.x, v2.y - v0.y, v2.z - v0.z);
        let normal_y = edge1.0 * edge2.2 - edge1.2 * edge2.0; // Cross product Y component

        let fill_color = if normal_y.abs() > 0.5 {
            // Floor/ceiling (horizontal face)
            Color::from_rgba(60, 120, 120, 100) // Cyan-ish
        } else {
            // Wall (vertical face)
            Color::from_rgba(100, 80, 60, 80) // Brown-ish
        };

        // Draw as two triangles (simple fill)
        // Note: macroquad doesn't have polygon fill, so we'll use triangles
        draw_triangle(
            Vec2::new(sx0, sy0),
            Vec2::new(sx1, sy1),
            Vec2::new(sx2, sy2),
            fill_color,
        );
        if !face.is_triangle {
            draw_triangle(
                Vec2::new(sx0, sy0),
                Vec2::new(sx2, sy2),
                Vec2::new(sx3, sy3),
                fill_color,
            );
        }

        // Highlight selected face
        if let super::Selection::Face { room: _, face: sel_face } = state.selection {
            if sel_face == face_idx {
                draw_triangle(
                    Vec2::new(sx0, sy0),
                    Vec2::new(sx1, sy1),
                    Vec2::new(sx2, sy2),
                    Color::from_rgba(255, 200, 100, 100),
                );
            }
        }
    }

    // Second pass: draw edges
    for face in &room.faces {
        let indices = if face.is_triangle {
            vec![0, 1, 2, 0]
        } else {
            vec![0, 1, 2, 3, 0]
        };

        for i in 0..indices.len() - 1 {
            let v0 = room.vertices[face.indices[indices[i]]];
            let v1 = room.vertices[face.indices[indices[i + 1]]];

            let (sx0, sy0) = world_to_screen(v0.x, v0.z);
            let (sx1, sy1) = world_to_screen(v1.x, v1.z);

            draw_line(sx0, sy0, sx1, sy1, 1.0, Color::from_rgba(150, 150, 160, 255));
        }
    }

    // Draw portals
    for portal in &room.portals {
        let v0 = portal.vertices[0];
        let v1 = portal.vertices[1];
        let v2 = portal.vertices[2];
        let v3 = portal.vertices[3];

        let (sx0, sy0) = world_to_screen(v0.x, v0.z);
        let (sx1, sy1) = world_to_screen(v1.x, v1.z);
        let (sx2, sy2) = world_to_screen(v2.x, v2.z);
        let (sx3, sy3) = world_to_screen(v3.x, v3.z);

        // Portal fill (magenta)
        draw_triangle(
            Vec2::new(sx0, sy0),
            Vec2::new(sx1, sy1),
            Vec2::new(sx2, sy2),
            Color::from_rgba(200, 50, 200, 80),
        );
        draw_triangle(
            Vec2::new(sx0, sy0),
            Vec2::new(sx2, sy2),
            Vec2::new(sx3, sy3),
            Color::from_rgba(200, 50, 200, 80),
        );

        // Portal outline
        draw_line(sx0, sy0, sx1, sy1, 2.0, Color::from_rgba(255, 100, 255, 255));
        draw_line(sx1, sy1, sx2, sy2, 2.0, Color::from_rgba(255, 100, 255, 255));
        draw_line(sx2, sy2, sx3, sy3, 2.0, Color::from_rgba(255, 100, 255, 255));
        draw_line(sx3, sy3, sx0, sy0, 2.0, Color::from_rgba(255, 100, 255, 255));
    }

    // Find vertex under mouse cursor (for selection/dragging)
    let mut hovered_vertex: Option<usize> = None;
    for (i, v) in room.vertices.iter().enumerate() {
        let (sx, sy) = world_to_screen(v.x, v.z);
        let dist = ((mouse_pos.0 - sx).powi(2) + (mouse_pos.1 - sy).powi(2)).sqrt();
        if dist < 8.0 {
            hovered_vertex = Some(i);
            break;
        }
    }

    // Store room index for later mutation
    let current_room_idx = state.current_room;

    // Draw vertices
    for (i, v) in room.vertices.iter().enumerate() {
        let (sx, sy) = world_to_screen(v.x, v.z);

        // Skip if outside view
        if sx < rect.x - 5.0 || sx > rect.right() + 5.0 || sy < rect.y - 5.0 || sy > rect.bottom() + 5.0 {
            continue;
        }

        let is_selected = matches!(state.selection, Selection::Vertex { vertex, .. } if vertex == i);
        let is_hovered = hovered_vertex == Some(i);
        let is_dragging = state.grid_dragging_vertex == Some(i);

        let color = if is_dragging {
            Color::from_rgba(100, 255, 100, 255) // Green while dragging
        } else if is_selected {
            Color::from_rgba(255, 255, 100, 255) // Yellow when selected
        } else if is_hovered {
            Color::from_rgba(255, 200, 150, 255) // Orange when hovered
        } else {
            Color::from_rgba(200, 200, 220, 255) // Default
        };

        let radius = if is_hovered || is_selected || is_dragging { 5.0 } else { 3.0 };
        draw_circle(sx, sy, radius, color);
    }

    // Draw room origin marker
    let (ox, oy) = world_to_screen(0.0, 0.0);
    if ox >= rect.x && ox <= rect.right() && oy >= rect.y && oy <= rect.bottom() {
        draw_circle(ox, oy, 5.0, Color::from_rgba(255, 100, 100, 255));
    }

    // Handle vertex selection and dragging (only with left mouse, and not panning)
    if inside && !state.grid_panning {
        // Start dragging on left press
        if ctx.mouse.left_pressed {
            if let Some(vi) = hovered_vertex {
                // Select and start dragging
                state.selection = Selection::Vertex { room: current_room_idx, vertex: vi };
                state.grid_dragging_vertex = Some(vi);
                state.grid_drag_started = false;
            } else {
                // Clicked empty space - deselect
                state.selection = Selection::None;
            }
        }

        // Continue dragging
        if ctx.mouse.left_down {
            if let Some(vi) = state.grid_dragging_vertex {
                // Save undo state on first actual movement
                if !state.grid_drag_started {
                    let dx = mouse_pos.0 - state.grid_last_mouse.0;
                    let dy = mouse_pos.1 - state.grid_last_mouse.1;
                    if dx.abs() > 1.0 || dy.abs() > 1.0 {
                        state.save_undo();
                        state.grid_drag_started = true;
                    }
                }

                // Move vertex to mouse position (convert screen to world)
                let wx = (mouse_pos.0 - center_x) / scale;
                let wz = (mouse_pos.1 - center_y) / scale;

                // Snap to grid if enabled
                let (snapped_x, snapped_z) = if state.show_grid {
                    let snap = state.grid_size;
                    ((wx / snap).round() * snap, (wz / snap).round() * snap)
                } else {
                    (wx, wz)
                };

                // Update vertex position
                if let Some(room) = state.level.rooms.get_mut(current_room_idx) {
                    if let Some(v) = room.vertices.get_mut(vi) {
                        v.x = snapped_x;
                        v.z = snapped_z;
                    }
                }
            }
        }

        // End dragging on release
        if ctx.mouse.left_released {
            state.grid_dragging_vertex = None;
            state.grid_drag_started = false;
        }
    } else {
        // Mouse left the rect or started panning - cancel drag
        if !ctx.mouse.left_down {
            state.grid_dragging_vertex = None;
            state.grid_drag_started = false;
        }
    }
}
