//! 3D Viewport - Software rendered preview

use macroquad::prelude::*;
use crate::ui::{Rect, UiContext};
use crate::rasterizer::{
    Framebuffer, Texture as RasterTexture, RasterSettings, render_mesh, Color as RasterColor,
    ray_triangle_intersect, screen_to_ray,
};
use super::{EditorState, Selection};

/// Draw the 3D viewport using the software rasterizer
pub fn draw_viewport_3d(
    ctx: &mut UiContext,
    rect: Rect,
    state: &mut EditorState,
    textures: &[RasterTexture],
    fb: &mut Framebuffer,
    settings: &RasterSettings,
) {
    let mouse_pos = (ctx.mouse.x, ctx.mouse.y);
    let inside_viewport = ctx.mouse.inside(&rect);

    // Camera rotation with right mouse button (same as game mode)
    if ctx.mouse.right_down && inside_viewport {
        if state.viewport_mouse_captured {
            // Calculate delta from last position
            let dx = -(mouse_pos.1 - state.viewport_last_mouse.1) * 0.005;
            let dy = (mouse_pos.0 - state.viewport_last_mouse.0) * 0.005;
            state.camera_3d.rotate(dx, dy);
        }
        state.viewport_mouse_captured = true;
    } else {
        state.viewport_mouse_captured = false;
    }
    state.viewport_last_mouse = mouse_pos;

    // Keyboard camera movement (WASD + Q/E) - only when viewport focused (right click or inside)
    let move_speed = 0.1;
    if inside_viewport || state.viewport_mouse_captured {
        if is_key_down(KeyCode::W) {
            state.camera_3d.position = state.camera_3d.position + state.camera_3d.basis_z * move_speed;
        }
        if is_key_down(KeyCode::S) {
            state.camera_3d.position = state.camera_3d.position - state.camera_3d.basis_z * move_speed;
        }
        if is_key_down(KeyCode::A) {
            state.camera_3d.position = state.camera_3d.position - state.camera_3d.basis_x * move_speed;
        }
        if is_key_down(KeyCode::D) {
            state.camera_3d.position = state.camera_3d.position + state.camera_3d.basis_x * move_speed;
        }
        if is_key_down(KeyCode::Q) {
            state.camera_3d.position = state.camera_3d.position - state.camera_3d.basis_y * move_speed;
        }
        if is_key_down(KeyCode::E) {
            state.camera_3d.position = state.camera_3d.position + state.camera_3d.basis_y * move_speed;
        }
    }

    // Face picking on left-click
    if ctx.mouse.clicked(&rect) && !ctx.mouse.right_down {
        // Convert screen mouse position to framebuffer coordinates
        // Account for aspect-correct scaling
        let fb_aspect = fb.width as f32 / fb.height as f32;
        let rect_aspect = rect.w / rect.h;

        let (draw_w, draw_h, draw_x, draw_y) = if fb_aspect > rect_aspect {
            let w = rect.w;
            let h = rect.w / fb_aspect;
            (w, h, rect.x, rect.y + (rect.h - h) * 0.5)
        } else {
            let h = rect.h;
            let w = rect.h * fb_aspect;
            (w, h, rect.x + (rect.w - w) * 0.5, rect.y)
        };

        // Check if mouse is within the actual rendered area
        let mx = mouse_pos.0;
        let my = mouse_pos.1;
        if mx >= draw_x && mx < draw_x + draw_w && my >= draw_y && my < draw_y + draw_h {
            // Convert to framebuffer pixel coordinates
            let fb_x = ((mx - draw_x) / draw_w * fb.width as f32) as f32;
            let fb_y = ((my - draw_y) / draw_h * fb.height as f32) as f32;

            // Cast ray from camera
            let (ray_origin, ray_dir) = screen_to_ray(
                fb_x,
                fb_y,
                fb.width,
                fb.height,
                state.camera_3d.position,
                state.camera_3d.basis_x,
                state.camera_3d.basis_y,
                state.camera_3d.basis_z,
            );

            // Test ray against all room faces
            let mut best_hit: Option<(usize, usize, f32)> = None; // (room_idx, face_idx, distance)

            for (room_idx, room) in state.level.rooms.iter().enumerate() {
                for (face_idx, face) in room.faces.iter().enumerate() {
                    // Get world-space vertices
                    let v0 = room.vertices[face.indices[0]] + room.position;
                    let v1 = room.vertices[face.indices[1]] + room.position;
                    let v2 = room.vertices[face.indices[2]] + room.position;
                    let v3 = room.vertices[face.indices[3]] + room.position;

                    // Test first triangle (v0, v1, v2)
                    if let Some(t) = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2) {
                        if best_hit.map_or(true, |(_, _, best_t)| t < best_t) {
                            best_hit = Some((room_idx, face_idx, t));
                        }
                    }

                    // Test second triangle for quads (v0, v2, v3)
                    if !face.is_triangle {
                        if let Some(t) = ray_triangle_intersect(ray_origin, ray_dir, v0, v2, v3) {
                            if best_hit.map_or(true, |(_, _, best_t)| t < best_t) {
                                best_hit = Some((room_idx, face_idx, t));
                            }
                        }
                    }
                }
            }

            // Select the closest hit face and apply texture
            if let Some((room_idx, face_idx, _)) = best_hit {
                state.selection = Selection::Face { room: room_idx, face: face_idx };
                let texture_id = state.selected_texture;
                state.save_undo();
                if let Some(room) = state.level.rooms.get_mut(room_idx) {
                    if let Some(face) = room.faces.get_mut(face_idx) {
                        face.texture_id = texture_id;
                    }
                }
            }
        }
    }

    // Clear framebuffer
    fb.clear(RasterColor::new(30, 30, 40));

    // Render all rooms
    for room in &state.level.rooms {
        let (vertices, faces) = room.to_render_data();
        render_mesh(fb, &vertices, &faces, textures, &state.camera_3d, settings);
    }

    // Store selected face edges for later drawing as 2D overlay
    let mut selected_face_screen_verts: Option<(Vec<(f32, f32)>, bool)> = None;

    if let Selection::Face { room: room_idx, face: face_idx } = state.selection {
        if let Some(room) = state.level.rooms.get(room_idx) {
            if let Some(face) = room.faces.get(face_idx) {
                // Get world-space vertices
                let v0 = room.vertices[face.indices[0]] + room.position;
                let v1 = room.vertices[face.indices[1]] + room.position;
                let v2 = room.vertices[face.indices[2]] + room.position;
                let v3 = room.vertices[face.indices[3]] + room.position;

                // Project vertices to screen space using camera transform
                let world_verts = if face.is_triangle {
                    vec![v0, v1, v2]
                } else {
                    vec![v0, v1, v2, v3]
                };

                let mut screen_verts = Vec::new();
                let mut all_visible = true;

                for v in &world_verts {
                    // Transform to camera space
                    let rel = *v - state.camera_3d.position;
                    let cam_x = rel.dot(state.camera_3d.basis_x);
                    let cam_y = rel.dot(state.camera_3d.basis_y);
                    let cam_z = rel.dot(state.camera_3d.basis_z);

                    // Check if behind camera
                    if cam_z <= 0.1 {
                        all_visible = false;
                        break;
                    }

                    // Project to screen (matching the rasterizer's projection)
                    const SCALE: f32 = 0.75;
                    let vs = (fb.width.min(fb.height) as f32 / 2.0) * SCALE;
                    let ud = 5.0;
                    let us = ud - 1.0;

                    let denom = cam_z + ud;
                    let sx = (cam_x * us / denom) * vs + (fb.width as f32 / 2.0);
                    let sy = (cam_y * us / denom) * vs + (fb.height as f32 / 2.0);

                    screen_verts.push((sx, sy));
                }

                if all_visible && screen_verts.len() >= 3 {
                    selected_face_screen_verts = Some((screen_verts, face.is_triangle));
                }
            }
        }
    }

    // Convert framebuffer to texture and draw to viewport
    let texture = Texture2D::from_rgba8(fb.width as u16, fb.height as u16, &fb.pixels);
    texture.set_filter(FilterMode::Nearest);

    // Calculate aspect-correct scaling
    let fb_aspect = fb.width as f32 / fb.height as f32;
    let rect_aspect = rect.w / rect.h;

    let (draw_w, draw_h, draw_x, draw_y) = if fb_aspect > rect_aspect {
        // Framebuffer is wider - fit to width
        let w = rect.w;
        let h = rect.w / fb_aspect;
        (w, h, rect.x, rect.y + (rect.h - h) * 0.5)
    } else {
        // Framebuffer is taller - fit to height
        let h = rect.h;
        let w = rect.h * fb_aspect;
        (w, h, rect.x + (rect.w - w) * 0.5, rect.y)
    };

    draw_texture_ex(
        &texture,
        draw_x,
        draw_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(draw_w, draw_h)),
            ..Default::default()
        },
    );

    // Draw viewport border
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::from_rgba(60, 60, 60, 255));

    // Draw selected face outline as 2D overlay
    if let Some((screen_verts, _is_triangle)) = selected_face_screen_verts {
        // Convert framebuffer coordinates to screen coordinates
        let to_screen = |fb_x: f32, fb_y: f32| -> (f32, f32) {
            let screen_x = draw_x + (fb_x / fb.width as f32) * draw_w;
            let screen_y = draw_y + (fb_y / fb.height as f32) * draw_h;
            (screen_x, screen_y)
        };

        let highlight_color = Color::from_rgba(255, 200, 50, 255);
        let n = screen_verts.len();

        // Draw edges
        for i in 0..n {
            let (x1, y1) = to_screen(screen_verts[i].0, screen_verts[i].1);
            let (x2, y2) = to_screen(screen_verts[(i + 1) % n].0, screen_verts[(i + 1) % n].1);
            draw_line(x1, y1, x2, y2, 2.0, highlight_color);
        }

        // Draw corner markers
        for (fx, fy) in &screen_verts {
            let (sx, sy) = to_screen(*fx, *fy);
            draw_circle(sx, sy, 4.0, highlight_color);
        }
    }

    // Draw camera info
    draw_text(
        &format!(
            "Cam: ({:.1}, {:.1}, {:.1})",
            state.camera_3d.position.x,
            state.camera_3d.position.y,
            state.camera_3d.position.z
        ),
        rect.x + 5.0,
        rect.bottom() - 5.0,
        12.0,
        Color::from_rgba(150, 150, 150, 255),
    );
}
