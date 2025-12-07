//! 3D Viewport - Software rendered preview
//!
//! Sector-based geometry system - selection works on faces within sectors.

use macroquad::prelude::*;
use crate::ui::{Rect, UiContext};
use crate::rasterizer::{
    Framebuffer, Texture as RasterTexture, render_mesh, Color as RasterColor, Vec3,
    WIDTH, HEIGHT, WIDTH_HI, HEIGHT_HI,
    perspective_transform,
};
use crate::world::SECTOR_SIZE;
use super::{EditorState, EditorTool, Selection, SectorFace};

/// Project a world-space point to framebuffer coordinates
fn world_to_screen(
    world_pos: Vec3,
    camera_pos: Vec3,
    basis_x: Vec3,
    basis_y: Vec3,
    basis_z: Vec3,
    fb_width: usize,
    fb_height: usize,
) -> Option<(f32, f32)> {
    let rel = world_pos - camera_pos;
    let cam_z = rel.dot(basis_z);

    // Behind camera
    if cam_z <= 0.1 {
        return None;
    }

    let cam_x = rel.dot(basis_x);
    let cam_y = rel.dot(basis_y);

    // Same projection as the rasterizer
    const SCALE: f32 = 0.75;
    let vs = (fb_width.min(fb_height) as f32 / 2.0) * SCALE;
    let ud = 5.0;
    let us = ud - 1.0;

    let denom = cam_z + ud;
    let sx = (cam_x * us / denom) * vs + (fb_width as f32 / 2.0);
    let sy = (cam_y * us / denom) * vs + (fb_height as f32 / 2.0);

    Some((sx, sy))
}

/// Calculate distance from point to line segment in 2D screen space
fn point_to_segment_distance(
    px: f32, py: f32,      // Point
    x1: f32, y1: f32,      // Segment start
    x2: f32, y2: f32,      // Segment end
) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-6 {
        // Segment is essentially a point
        let pdx = px - x1;
        let pdy = py - y1;
        return (pdx * pdx + pdy * pdy).sqrt();
    }

    // Project point onto line segment
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    // Find closest point on segment
    let closest_x = x1 + t * dx;
    let closest_y = y1 + t * dy;

    // Distance from point to closest point
    let dist_x = px - closest_x;
    let dist_y = py - closest_y;
    (dist_x * dist_x + dist_y * dist_y).sqrt()
}

/// Test if point is inside 2D triangle using sign-based edge test
/// This works regardless of triangle winding order
fn point_in_triangle_2d(
    px: f32, py: f32,      // Point
    x1: f32, y1: f32,      // Triangle v1
    x2: f32, y2: f32,      // Triangle v2
    x3: f32, y3: f32,      // Triangle v3
) -> bool {
    // Calculate signed areas using cross product
    fn sign(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
        (px - bx) * (ay - by) - (ax - bx) * (py - by)
    }

    let d1 = sign(px, py, x1, y1, x2, y2);
    let d2 = sign(px, py, x2, y2, x3, y3);
    let d3 = sign(px, py, x3, y3, x1, y1);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    // Point is inside if all signs are same (all positive or all negative)
    !(has_neg && has_pos)
}

/// Draw the 3D viewport using the software rasterizer
pub fn draw_viewport_3d(
    ctx: &mut UiContext,
    rect: Rect,
    state: &mut EditorState,
    textures: &[RasterTexture],
    fb: &mut Framebuffer,
) {
    // Resize framebuffer based on resolution setting
    let (target_w, target_h) = if state.raster_settings.low_resolution {
        (WIDTH, HEIGHT)
    } else {
        (WIDTH_HI, HEIGHT_HI)
    };
    fb.resize(target_w, target_h);

    let mouse_pos = (ctx.mouse.x, ctx.mouse.y);
    let inside_viewport = ctx.mouse.inside(&rect);

    // Pre-calculate viewport scaling (used multiple times)
    let fb_width = fb.width;
    let fb_height = fb.height;
    let fb_aspect = fb_width as f32 / fb_height as f32;
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

    // Helper to convert screen mouse to framebuffer coordinates
    let screen_to_fb = |mx: f32, my: f32| -> Option<(f32, f32)> {
        if mx >= draw_x && mx < draw_x + draw_w && my >= draw_y && my < draw_y + draw_h {
            let fb_x = (mx - draw_x) / draw_w * fb_width as f32;
            let fb_y = (my - draw_y) / draw_h * fb_height as f32;
            Some((fb_x, fb_y))
        } else {
            None
        }
    };

    // Camera rotation with right mouse button (same as game mode)
    // Only rotate camera when not dragging a vertex
    if ctx.mouse.right_down && inside_viewport && state.dragging_sector_vertices.is_empty() {
        if state.viewport_mouse_captured {
            // Inverted to match Y-down coordinate system
            let dx = (mouse_pos.1 - state.viewport_last_mouse.1) * 0.005;
            let dy = -(mouse_pos.0 - state.viewport_last_mouse.0) * 0.005;
            state.camera_3d.rotate(dx, dy);
        }
        state.viewport_mouse_captured = true;
    } else if !ctx.mouse.right_down {
        state.viewport_mouse_captured = false;
    }

    // Keyboard camera movement (WASD + Q/E) - only when viewport focused and not dragging
    let move_speed = 100.0; // Scaled for TRLE units (1024 per sector)
    if (inside_viewport || state.viewport_mouse_captured) && state.dragging_sector_vertices.is_empty() {
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

    // Toggle link coincident vertices mode with L key
    if inside_viewport && is_key_pressed(KeyCode::L) {
        state.link_coincident_vertices = !state.link_coincident_vertices;
        let mode = if state.link_coincident_vertices { "Linked" } else { "Independent" };
        state.set_status(&format!("Vertex mode: {}", mode), 2.0);
    }

    // Find hovered elements using 2D screen-space projection
    // Priority: vertex > edge > face
    let mut hovered_vertex: Option<(usize, usize, usize, usize, f32)> = None; // (room_idx, gx, gz, corner_idx, screen_dist)
    let mut hovered_edge: Option<(usize, usize, usize, usize, usize, f32)> = None; // (room_idx, gx, gz, face_idx, edge_idx, dist)
    let mut hovered_face: Option<(usize, usize, usize, SectorFace)> = None; // (room_idx, gx, gz, face)
    let mut preview_sector: Option<(f32, f32, f32, bool)> = None; // (x, z, target_y, is_occupied)

    // Collect all vertex positions for the current room (for drawing and selection)
    // Each vertex is (world_pos, room_idx, gx, gz, corner_idx, face_type)
    // corner_idx: 0=NW, 1=NE, 2=SE, 3=SW for horizontal faces
    let mut all_vertices: Vec<(Vec3, usize, usize, usize, usize, SectorFace)> = Vec::new();

    if let Some(room) = state.level.rooms.get(state.current_room) {
        for (gx, gz, sector) in room.iter_sectors() {
            let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
            let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;

            // Floor vertices
            if let Some(floor) = &sector.floor {
                all_vertices.push((Vec3::new(base_x, floor.heights[0], base_z), state.current_room, gx, gz, 0, SectorFace::Floor));
                all_vertices.push((Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z), state.current_room, gx, gz, 1, SectorFace::Floor));
                all_vertices.push((Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE), state.current_room, gx, gz, 2, SectorFace::Floor));
                all_vertices.push((Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE), state.current_room, gx, gz, 3, SectorFace::Floor));
            }

            // Ceiling vertices
            if let Some(ceiling) = &sector.ceiling {
                all_vertices.push((Vec3::new(base_x, ceiling.heights[0], base_z), state.current_room, gx, gz, 0, SectorFace::Ceiling));
                all_vertices.push((Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z), state.current_room, gx, gz, 1, SectorFace::Ceiling));
                all_vertices.push((Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE), state.current_room, gx, gz, 2, SectorFace::Ceiling));
                all_vertices.push((Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE), state.current_room, gx, gz, 3, SectorFace::Ceiling));
            }

            // Wall vertices
            let wall_configs: [(&Vec<crate::world::VerticalFace>, f32, f32, f32, f32, fn(usize) -> SectorFace); 4] = [
                (&sector.walls_north, base_x, base_z, base_x + SECTOR_SIZE, base_z, |i| SectorFace::WallNorth(i)),
                (&sector.walls_east, base_x + SECTOR_SIZE, base_z, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, |i| SectorFace::WallEast(i)),
                (&sector.walls_south, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, base_x, base_z + SECTOR_SIZE, |i| SectorFace::WallSouth(i)),
                (&sector.walls_west, base_x, base_z + SECTOR_SIZE, base_x, base_z, |i| SectorFace::WallWest(i)),
            ];

            for (walls, x0, z0, x1, z1, make_face) in wall_configs {
                for (i, wall) in walls.iter().enumerate() {
                    // 4 corners of wall: bottom-left, bottom-right, top-right, top-left
                    all_vertices.push((Vec3::new(x0, wall.y_bottom, z0), state.current_room, gx, gz, 0, make_face(i)));
                    all_vertices.push((Vec3::new(x1, wall.y_bottom, z1), state.current_room, gx, gz, 1, make_face(i)));
                    all_vertices.push((Vec3::new(x1, wall.y_top, z1), state.current_room, gx, gz, 2, make_face(i)));
                    all_vertices.push((Vec3::new(x0, wall.y_top, z0), state.current_room, gx, gz, 3, make_face(i)));
                }
            }
        }
    }

    // In Select mode, find hovered vertex/edge/face using 2D screen projection
    if inside_viewport && !ctx.mouse.right_down && state.tool == EditorTool::Select {
        if let Some((mouse_fb_x, mouse_fb_y)) = screen_to_fb(mouse_pos.0, mouse_pos.1) {
            const VERTEX_THRESHOLD: f32 = 10.0;
            const EDGE_THRESHOLD: f32 = 8.0;

            // Check vertices first (highest priority)
            for (world_pos, room_idx, gx, gz, corner_idx, face) in &all_vertices {
                if let Some((sx, sy)) = world_to_screen(
                    *world_pos,
                    state.camera_3d.position,
                    state.camera_3d.basis_x,
                    state.camera_3d.basis_y,
                    state.camera_3d.basis_z,
                    fb.width,
                    fb.height,
                ) {
                    let dist = ((mouse_fb_x - sx).powi(2) + (mouse_fb_y - sy).powi(2)).sqrt();
                    if dist < VERTEX_THRESHOLD {
                        if hovered_vertex.map_or(true, |(_, _, _, _, best_dist)| dist < best_dist) {
                            hovered_vertex = Some((*room_idx, *gx, *gz, *corner_idx, dist));
                        }
                    }
                }
            }

            // Check edges if no vertex hovered
            if hovered_vertex.is_none() {
                if let Some(room) = state.level.rooms.get(state.current_room) {
                    for (gx, gz, sector) in room.iter_sectors() {
                        let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
                        let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;

                        // Check floor edges
                        if let Some(floor) = &sector.floor {
                            let corners = [
                                Vec3::new(base_x, floor.heights[0], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE),
                                Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE),
                            ];
                            for edge_idx in 0..4 {
                                let v0 = corners[edge_idx];
                                let v1 = corners[(edge_idx + 1) % 4];
                                if let (Some((sx0, sy0)), Some((sx1, sy1))) = (
                                    world_to_screen(v0, state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                    world_to_screen(v1, state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height)
                                ) {
                                    let dist = point_to_segment_distance(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1);
                                    if dist < EDGE_THRESHOLD {
                                        if hovered_edge.map_or(true, |(_, _, _, _, _, best_dist)| dist < best_dist) {
                                            hovered_edge = Some((state.current_room, gx, gz, 0, edge_idx, dist)); // face_idx=0 for floor
                                        }
                                    }
                                }
                            }
                        }

                        // Check ceiling edges
                        if let Some(ceiling) = &sector.ceiling {
                            let corners = [
                                Vec3::new(base_x, ceiling.heights[0], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE),
                                Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE),
                            ];
                            for edge_idx in 0..4 {
                                let v0 = corners[edge_idx];
                                let v1 = corners[(edge_idx + 1) % 4];
                                if let (Some((sx0, sy0)), Some((sx1, sy1))) = (
                                    world_to_screen(v0, state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                    world_to_screen(v1, state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height)
                                ) {
                                    let dist = point_to_segment_distance(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1);
                                    if dist < EDGE_THRESHOLD {
                                        if hovered_edge.map_or(true, |(_, _, _, _, _, best_dist)| dist < best_dist) {
                                            hovered_edge = Some((state.current_room, gx, gz, 1, edge_idx, dist)); // face_idx=1 for ceiling
                                        }
                                    }
                                }
                            }
                        }

                        // Check wall edges
                        let wall_configs: [(&Vec<crate::world::VerticalFace>, f32, f32, f32, f32, fn(usize) -> SectorFace); 4] = [
                            (&sector.walls_north, base_x, base_z, base_x + SECTOR_SIZE, base_z, |i| SectorFace::WallNorth(i)),
                            (&sector.walls_east, base_x + SECTOR_SIZE, base_z, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, |i| SectorFace::WallEast(i)),
                            (&sector.walls_south, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, base_x, base_z + SECTOR_SIZE, |i| SectorFace::WallSouth(i)),
                            (&sector.walls_west, base_x, base_z + SECTOR_SIZE, base_x, base_z, |i| SectorFace::WallWest(i)),
                        ];

                        for (walls, x0, z0, x1, z1, _make_face) in wall_configs {
                            for (_i, wall) in walls.iter().enumerate() {
                                let wall_corners = [
                                    Vec3::new(x0, wall.y_bottom, z0),
                                    Vec3::new(x1, wall.y_bottom, z1),
                                    Vec3::new(x1, wall.y_top, z1),
                                    Vec3::new(x0, wall.y_top, z0),
                                ];
                                for edge_idx in 0..4 {
                                    let v0 = wall_corners[edge_idx];
                                    let v1 = wall_corners[(edge_idx + 1) % 4];
                                    if let (Some((sx0, sy0)), Some((sx1, sy1))) = (
                                        world_to_screen(v0, state.camera_3d.position, state.camera_3d.basis_x,
                                            state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                        world_to_screen(v1, state.camera_3d.position, state.camera_3d.basis_x,
                                            state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height)
                                    ) {
                                        let dist = point_to_segment_distance(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1);
                                        if dist < EDGE_THRESHOLD {
                                            if hovered_edge.map_or(true, |(_, _, _, _, _, best_dist)| dist < best_dist) {
                                                hovered_edge = Some((state.current_room, gx, gz, 2, edge_idx, dist)); // face_idx=2 for walls
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check faces if no vertex or edge hovered
            if hovered_vertex.is_none() && hovered_edge.is_none() {
                if let Some(room) = state.level.rooms.get(state.current_room) {
                    'face_loop: for (gx, gz, sector) in room.iter_sectors() {
                        let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
                        let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;

                        // Check floor (no backface culling - always selectable)
                        if let Some(floor) = &sector.floor {
                            let corners = [
                                Vec3::new(base_x, floor.heights[0], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE),
                                Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE),
                            ];

                            if let (Some((sx0, sy0)), Some((sx1, sy1)), Some((sx2, sy2)), Some((sx3, sy3))) = (
                                world_to_screen(corners[0], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[1], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[2], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[3], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                            ) {
                                // Test both triangles that make up the quad (0-1-2 and 0-2-3)
                                if point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1, sx2, sy2) ||
                                   point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx2, sy2, sx3, sy3) {
                                    hovered_face = Some((state.current_room, gx, gz, SectorFace::Floor));
                                    break 'face_loop;
                                }
                            }
                        }

                        // Check ceiling
                        if let Some(ceiling) = &sector.ceiling {
                            let corners = [
                                Vec3::new(base_x, ceiling.heights[0], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z),
                                Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE),
                                Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE),
                            ];

                            if let (Some((sx0, sy0)), Some((sx1, sy1)), Some((sx2, sy2)), Some((sx3, sy3))) = (
                                world_to_screen(corners[0], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[1], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[2], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                world_to_screen(corners[3], state.camera_3d.position, state.camera_3d.basis_x,
                                    state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                            ) {
                                if point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1, sx2, sy2) ||
                                   point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx2, sy2, sx3, sy3) {
                                    hovered_face = Some((state.current_room, gx, gz, SectorFace::Ceiling));
                                    break 'face_loop;
                                }
                            }
                        }

                        // Check walls
                        let wall_configs: [(&Vec<crate::world::VerticalFace>, f32, f32, f32, f32, fn(usize) -> SectorFace); 4] = [
                            (&sector.walls_north, base_x, base_z, base_x + SECTOR_SIZE, base_z, |i| SectorFace::WallNorth(i)),
                            (&sector.walls_east, base_x + SECTOR_SIZE, base_z, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, |i| SectorFace::WallEast(i)),
                            (&sector.walls_south, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, base_x, base_z + SECTOR_SIZE, |i| SectorFace::WallSouth(i)),
                            (&sector.walls_west, base_x, base_z + SECTOR_SIZE, base_x, base_z, |i| SectorFace::WallWest(i)),
                        ];

                        for (walls, x0, z0, x1, z1, make_face) in wall_configs {
                            for (i, wall) in walls.iter().enumerate() {
                                let wall_corners = [
                                    Vec3::new(x0, wall.y_bottom, z0),
                                    Vec3::new(x1, wall.y_bottom, z1),
                                    Vec3::new(x1, wall.y_top, z1),
                                    Vec3::new(x0, wall.y_top, z0),
                                ];

                                if let (Some((sx0, sy0)), Some((sx1, sy1)), Some((sx2, sy2)), Some((sx3, sy3))) = (
                                    world_to_screen(wall_corners[0], state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                    world_to_screen(wall_corners[1], state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                    world_to_screen(wall_corners[2], state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                    world_to_screen(wall_corners[3], state.camera_3d.position, state.camera_3d.basis_x,
                                        state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                                ) {
                                    if point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx1, sy1, sx2, sy2) ||
                                       point_in_triangle_2d(mouse_fb_x, mouse_fb_y, sx0, sy0, sx2, sy2, sx3, sy3) {
                                        hovered_face = Some((state.current_room, gx, gz, make_face(i)));
                                        break 'face_loop;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // In drawing modes, find preview sector position
    if inside_viewport && (state.tool == EditorTool::DrawFloor || state.tool == EditorTool::DrawCeiling) {
        if let Some((mouse_fb_x, mouse_fb_y)) = screen_to_fb(mouse_pos.0, mouse_pos.1) {
            use super::{CEILING_HEIGHT, CLICK_HEIGHT};

            let is_floor = state.tool == EditorTool::DrawFloor;

            // Handle Shift+drag for height adjustment
            let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

            if shift_down && !state.height_adjust_mode {
                // Just started holding shift - enter height adjust mode
                state.height_adjust_mode = true;
                state.height_adjust_start_mouse_y = mouse_pos.1;
                state.height_adjust_start_y = state.placement_target_y;
            } else if !shift_down && state.height_adjust_mode {
                // Released shift - exit height adjust mode
                state.height_adjust_mode = false;
            }

            // Adjust height while shift is held
            if state.height_adjust_mode {
                let mouse_delta = state.height_adjust_start_mouse_y - mouse_pos.1;
                let y_sensitivity = 5.0;
                let raw_delta = mouse_delta * y_sensitivity;
                // Snap to CLICK_HEIGHT increments
                let snapped_delta = (raw_delta / CLICK_HEIGHT).round() * CLICK_HEIGHT;
                state.placement_target_y = state.height_adjust_start_y + snapped_delta;
                // Show height in status bar
                let clicks = (state.placement_target_y / CLICK_HEIGHT) as i32;
                state.set_status(&format!("Height: {:.0} ({} clicks)", state.placement_target_y, clicks), 0.5);
            }

            // Use placement_target_y, but initialize to sensible default if zero
            let target_y = if state.placement_target_y == 0.0 && !state.height_adjust_mode {
                // Default: floor at 0, ceiling at CEILING_HEIGHT
                if is_floor { 0.0 } else { CEILING_HEIGHT }
            } else {
                state.placement_target_y
            };

            let search_radius = 20;
            let cam_x = state.camera_3d.position.x;
            let cam_z = state.camera_3d.position.z;
            let start_x = ((cam_x / SECTOR_SIZE).floor() as i32 - search_radius) as f32 * SECTOR_SIZE;
            let start_z = ((cam_z / SECTOR_SIZE).floor() as i32 - search_radius) as f32 * SECTOR_SIZE;

            let mut closest: Option<(f32, f32, f32)> = None;
            for ix in 0..(search_radius * 2) {
                for iz in 0..(search_radius * 2) {
                    let grid_x = start_x + (ix as f32 * SECTOR_SIZE);
                    let grid_z = start_z + (iz as f32 * SECTOR_SIZE);
                    let test_pos = Vec3::new(grid_x + SECTOR_SIZE / 2.0, target_y, grid_z + SECTOR_SIZE / 2.0);

                    if let Some((sx, sy)) = world_to_screen(test_pos, state.camera_3d.position,
                        state.camera_3d.basis_x, state.camera_3d.basis_y, state.camera_3d.basis_z,
                        fb.width, fb.height)
                    {
                        let dist = ((mouse_fb_x - sx).powi(2) + (mouse_fb_y - sy).powi(2)).sqrt();
                        if closest.map_or(true, |(_, _, best_dist)| dist < best_dist) {
                            closest = Some((grid_x, grid_z, dist));
                        }
                    }
                }
            }

            if let Some((snapped_x, snapped_z, dist)) = closest {
                if dist < 100.0 {
                    // Check if sector is occupied using new sector API
                    let occupied = if let Some(room) = state.level.rooms.get(state.current_room) {
                        // Convert world coords to grid coords
                        if let Some((gx, gz)) = room.world_to_grid(snapped_x + SECTOR_SIZE * 0.5, snapped_z + SECTOR_SIZE * 0.5) {
                            if let Some(sector) = room.get_sector(gx, gz) {
                                if is_floor { sector.floor.is_some() } else { sector.ceiling.is_some() }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    preview_sector = Some((snapped_x, snapped_z, target_y, occupied));
                }
            }
        }
    }

    // Handle clicks and dragging in 3D viewport
    if inside_viewport && !ctx.mouse.right_down {
        // Start dragging or select on left press
        if ctx.mouse.left_pressed {
            if state.tool == EditorTool::Select {
                // Priority: vertex > edge > face
                if let Some((room_idx, gx, gz, corner_idx, _)) = hovered_vertex {
                    // Start dragging vertex
                    state.dragging_sector_vertices.clear();
                    state.drag_initial_heights.clear();
                    state.viewport_drag_started = false;

                    // Get the face type for this vertex from all_vertices
                    if let Some((_, _, _, _, _, face)) = all_vertices.iter()
                        .find(|(_, ri, vgx, vgz, ci, _)| *ri == room_idx && *vgx == gx && *vgz == gz && *ci == corner_idx)
                    {
                        // Store the vertex to drag
                        state.dragging_sector_vertices.push((room_idx, gx, gz, *face, corner_idx));

                        // Get initial height
                        if let Some(room) = state.level.rooms.get(room_idx) {
                            if let Some(sector) = room.get_sector(gx, gz) {
                                let height = match face {
                                    SectorFace::Floor => sector.floor.as_ref().map(|f| f.heights[corner_idx]),
                                    SectorFace::Ceiling => sector.ceiling.as_ref().map(|c| c.heights[corner_idx]),
                                    SectorFace::WallNorth(i) => sector.walls_north.get(*i).map(|w| if corner_idx < 2 { w.y_bottom } else { w.y_top }),
                                    SectorFace::WallEast(i) => sector.walls_east.get(*i).map(|w| if corner_idx < 2 { w.y_bottom } else { w.y_top }),
                                    SectorFace::WallSouth(i) => sector.walls_south.get(*i).map(|w| if corner_idx < 2 { w.y_bottom } else { w.y_top }),
                                    SectorFace::WallWest(i) => sector.walls_west.get(*i).map(|w| if corner_idx < 2 { w.y_bottom } else { w.y_top }),
                                };
                                if let Some(h) = height {
                                    state.drag_initial_heights.push(h);
                                    state.viewport_drag_plane_y = h;
                                }
                            }
                        }

                        // If linking mode is on, find coincident vertices
                        if state.link_coincident_vertices {
                            // Get position of clicked vertex
                            if let Some((world_pos, _, _, _, _, _)) = all_vertices.iter()
                                .find(|(_, ri, vgx, vgz, ci, f)| *ri == room_idx && *vgx == gx && *vgz == gz && *ci == corner_idx && f == face)
                            {
                                const EPSILON: f32 = 0.1;
                                // Find all vertices at same position
                                for (pos, ri, vgx, vgz, ci, vface) in &all_vertices {
                                    if (pos.x - world_pos.x).abs() < EPSILON &&
                                       (pos.y - world_pos.y).abs() < EPSILON &&
                                       (pos.z - world_pos.z).abs() < EPSILON {
                                        let key = (*ri, *vgx, *vgz, *vface, *ci);
                                        if !state.dragging_sector_vertices.contains(&key) {
                                            state.dragging_sector_vertices.push(key);
                                            state.drag_initial_heights.push(pos.y);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some((room_idx, gx, gz, face_idx, edge_idx, _)) = hovered_edge {
                    // Start dragging edge (both vertices)
                    state.dragging_sector_vertices.clear();
                    state.drag_initial_heights.clear();
                    state.viewport_drag_started = false;

                    // Determine face type and get both edge vertices
                    let face_type = match face_idx {
                        0 => Some(SectorFace::Floor),
                        1 => Some(SectorFace::Ceiling),
                        _ => None, // TODO: wall edges
                    };

                    if let Some(face) = face_type {
                        let corner0 = edge_idx;
                        let corner1 = (edge_idx + 1) % 4;

                        if let Some(room) = state.level.rooms.get(room_idx) {
                            if let Some(sector) = room.get_sector(gx, gz) {
                                let heights = match face {
                                    SectorFace::Floor => sector.floor.as_ref().map(|f| f.heights),
                                    SectorFace::Ceiling => sector.ceiling.as_ref().map(|c| c.heights),
                                    _ => None,
                                };
                                if let Some(h) = heights {
                                    state.dragging_sector_vertices.push((room_idx, gx, gz, face, corner0));
                                    state.dragging_sector_vertices.push((room_idx, gx, gz, face, corner1));
                                    state.drag_initial_heights.push(h[corner0]);
                                    state.drag_initial_heights.push(h[corner1]);
                                    state.viewport_drag_plane_y = (h[corner0] + h[corner1]) / 2.0;
                                }
                            }
                        }
                    }

                    state.selection = Selection::Sector { room: room_idx, x: gx, z: gz };
                } else if let Some((room_idx, gx, gz, face)) = hovered_face {
                    // Start dragging face (all 4 vertices)
                    state.dragging_sector_vertices.clear();
                    state.drag_initial_heights.clear();
                    state.viewport_drag_started = false;

                    if let Some(room) = state.level.rooms.get(room_idx) {
                        if let Some(sector) = room.get_sector(gx, gz) {
                            let heights = match face {
                                SectorFace::Floor => sector.floor.as_ref().map(|f| f.heights),
                                SectorFace::Ceiling => sector.ceiling.as_ref().map(|c| c.heights),
                                _ => None, // Walls have 2 heights (y_bottom, y_top)
                            };

                            if let Some(h) = heights {
                                for corner in 0..4 {
                                    state.dragging_sector_vertices.push((room_idx, gx, gz, face, corner));
                                    state.drag_initial_heights.push(h[corner]);
                                }
                                state.viewport_drag_plane_y = (h[0] + h[1] + h[2] + h[3]) / 4.0;

                                // If linking, find coincident vertices
                                if state.link_coincident_vertices {
                                    let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
                                    let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;
                                    let face_positions = [
                                        Vec3::new(base_x, h[0], base_z),
                                        Vec3::new(base_x + SECTOR_SIZE, h[1], base_z),
                                        Vec3::new(base_x + SECTOR_SIZE, h[2], base_z + SECTOR_SIZE),
                                        Vec3::new(base_x, h[3], base_z + SECTOR_SIZE),
                                    ];

                                    const EPSILON: f32 = 0.1;
                                    for (pos, ri, vgx, vgz, ci, vface) in &all_vertices {
                                        for fp in &face_positions {
                                            if (pos.x - fp.x).abs() < EPSILON &&
                                               (pos.y - fp.y).abs() < EPSILON &&
                                               (pos.z - fp.z).abs() < EPSILON {
                                                let key = (*ri, *vgx, *vgz, *vface, *ci);
                                                if !state.dragging_sector_vertices.contains(&key) {
                                                    state.dragging_sector_vertices.push(key);
                                                    state.drag_initial_heights.push(pos.y);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Handle wall dragging (move both y_bottom and y_top)
                            match face {
                                SectorFace::WallNorth(i) | SectorFace::WallEast(i) |
                                SectorFace::WallSouth(i) | SectorFace::WallWest(i) => {
                                    let walls = match face {
                                        SectorFace::WallNorth(_) => &sector.walls_north,
                                        SectorFace::WallEast(_) => &sector.walls_east,
                                        SectorFace::WallSouth(_) => &sector.walls_south,
                                        SectorFace::WallWest(_) => &sector.walls_west,
                                        _ => unreachable!(),
                                    };
                                    if let Some(wall) = walls.get(i) {
                                        // For walls, corners 0,1 are bottom, 2,3 are top
                                        state.dragging_sector_vertices.push((room_idx, gx, gz, face, 0));
                                        state.dragging_sector_vertices.push((room_idx, gx, gz, face, 1));
                                        state.dragging_sector_vertices.push((room_idx, gx, gz, face, 2));
                                        state.dragging_sector_vertices.push((room_idx, gx, gz, face, 3));
                                        state.drag_initial_heights.push(wall.y_bottom);
                                        state.drag_initial_heights.push(wall.y_bottom);
                                        state.drag_initial_heights.push(wall.y_top);
                                        state.drag_initial_heights.push(wall.y_top);
                                        state.viewport_drag_plane_y = (wall.y_bottom + wall.y_top) / 2.0;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    state.selection = Selection::SectorFace { room: room_idx, x: gx, z: gz, face };
                } else {
                    // Clicked on nothing - clear selection
                    state.selection = Selection::None;
                }
            }
            // Drawing modes - place floor/ceiling
            else if state.tool == EditorTool::DrawFloor || state.tool == EditorTool::DrawCeiling {
                if let Some((snapped_x, snapped_z, target_y, occupied)) = preview_sector {
                    let is_floor = state.tool == EditorTool::DrawFloor;

                    if occupied {
                        let type_name = if is_floor { "floor" } else { "ceiling" };
                        state.set_status(&format!("Sector already has a {}", type_name), 2.0);
                    } else {
                        state.save_undo();

                        // Get texture and room position before borrowing mutably
                        let texture = state.selected_texture.clone();
                        let room_pos = state.level.rooms.get(state.current_room)
                            .map(|r| r.position)
                            .unwrap_or_default();

                        if let Some(room) = state.level.rooms.get_mut(state.current_room) {
                            // Convert world coords to grid coords
                            let gx = ((snapped_x - room_pos.x) / SECTOR_SIZE) as usize;
                            let gz = ((snapped_z - room_pos.z) / SECTOR_SIZE) as usize;

                            // Expand room grid if needed
                            while gx >= room.width {
                                room.width += 1;
                                room.sectors.push((0..room.depth).map(|_| None).collect());
                            }
                            while gz >= room.depth {
                                room.depth += 1;
                                for col in &mut room.sectors {
                                    col.push(None);
                                }
                            }

                            if is_floor {
                                room.set_floor(gx, gz, target_y, texture);
                            } else {
                                room.set_ceiling(gx, gz, target_y, texture);
                            }
                            room.recalculate_bounds();
                        }

                        let status = if is_floor { "Created floor sector" } else { "Created ceiling sector" };
                        state.set_status(status, 2.0);
                    }
                }
            }
        }

        // Continue dragging (Y-axis only - TRLE constraint)
        if ctx.mouse.left_down && !state.dragging_sector_vertices.is_empty() {
            use super::CLICK_HEIGHT;

            if !state.viewport_drag_started {
                state.save_undo();
                state.viewport_drag_started = true;
            }

            // Calculate Y delta from mouse movement (inverted: mouse up = positive Y)
            let mouse_delta_y = state.viewport_last_mouse.1 - mouse_pos.1;
            let y_sensitivity = 5.0;
            let y_delta = mouse_delta_y * y_sensitivity;

            // Accumulate delta
            state.viewport_drag_plane_y += y_delta;

            // Calculate delta from initial average
            let initial_avg: f32 = state.drag_initial_heights.iter().sum::<f32>()
                / state.drag_initial_heights.len().max(1) as f32;
            let delta_from_initial = state.viewport_drag_plane_y - initial_avg;

            // Apply delta to each vertex
            for (i, &(room_idx, gx, gz, face, corner_idx)) in state.dragging_sector_vertices.clone().iter().enumerate() {
                if let Some(initial_h) = state.drag_initial_heights.get(i) {
                    let new_h = initial_h + delta_from_initial;
                    let snapped_h = (new_h / CLICK_HEIGHT).round() * CLICK_HEIGHT;

                    if let Some(room) = state.level.rooms.get_mut(room_idx) {
                        if let Some(sector) = room.get_sector_mut(gx, gz) {
                            match face {
                                SectorFace::Floor => {
                                    if let Some(floor) = &mut sector.floor {
                                        floor.heights[corner_idx] = snapped_h;
                                    }
                                }
                                SectorFace::Ceiling => {
                                    if let Some(ceiling) = &mut sector.ceiling {
                                        ceiling.heights[corner_idx] = snapped_h;
                                    }
                                }
                                SectorFace::WallNorth(wi) | SectorFace::WallEast(wi) |
                                SectorFace::WallSouth(wi) | SectorFace::WallWest(wi) => {
                                    let walls = match face {
                                        SectorFace::WallNorth(_) => &mut sector.walls_north,
                                        SectorFace::WallEast(_) => &mut sector.walls_east,
                                        SectorFace::WallSouth(_) => &mut sector.walls_south,
                                        SectorFace::WallWest(_) => &mut sector.walls_west,
                                        _ => unreachable!(),
                                    };
                                    if let Some(wall) = walls.get_mut(wi) {
                                        if corner_idx < 2 {
                                            wall.y_bottom = snapped_h;
                                        } else {
                                            wall.y_top = snapped_h;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // End dragging on release
        if ctx.mouse.left_released {
            state.dragging_sector_vertices.clear();
            state.drag_initial_heights.clear();
            state.viewport_drag_started = false;
        }
    }

    // Update mouse position for next frame
    state.viewport_last_mouse = mouse_pos;

    // Clear framebuffer
    fb.clear(RasterColor::new(30, 30, 40));

    // Draw main floor grid (large, fixed extent)
    if state.show_grid {
        let grid_color = RasterColor::new(50, 50, 60);
        let grid_size = state.grid_size;
        let grid_extent = 10240.0; // Cover approximately 10 sectors in each direction

        // Calculate grid Y position based on lowest point in all rooms
        let grid_y = 0.0;

        // Draw grid lines - use shorter segments for better clipping behavior
        let segment_length: f32 = SECTOR_SIZE;

        // X-parallel lines (varying X, fixed Z)
        let mut z: f32 = -grid_extent;
        while z <= grid_extent {
            let mut x: f32 = -grid_extent;
            while x < grid_extent {
                let x_end = (x + segment_length).min(grid_extent);
                draw_3d_line(
                    fb,
                    Vec3::new(x, grid_y, z),
                    Vec3::new(x_end, grid_y, z),
                    &state.camera_3d,
                    grid_color,
                );
                x += segment_length;
            }
            z += grid_size;
        }

        // Z-parallel lines (fixed X, varying Z)
        let mut x: f32 = -grid_extent;
        while x <= grid_extent {
            let mut z: f32 = -grid_extent;
            while z < grid_extent {
                let z_end = (z + segment_length).min(grid_extent);
                draw_3d_line(
                    fb,
                    Vec3::new(x, grid_y, z),
                    Vec3::new(x, grid_y, z_end),
                    &state.camera_3d,
                    grid_color,
                );
                z += segment_length;
            }
            x += grid_size;
        }

        // Draw origin axes (slightly brighter)
        let mut x = -grid_extent;
        while x < grid_extent {
            let x_end = (x + segment_length).min(grid_extent);
            draw_3d_line(fb, Vec3::new(x, grid_y, 0.0), Vec3::new(x_end, grid_y, 0.0), &state.camera_3d, RasterColor::new(100, 60, 60));
            x += segment_length;
        }
        let mut z = -grid_extent;
        while z < grid_extent {
            let z_end = (z + segment_length).min(grid_extent);
            draw_3d_line(fb, Vec3::new(0.0, grid_y, z), Vec3::new(0.0, grid_y, z_end), &state.camera_3d, RasterColor::new(60, 60, 100));
            z += segment_length;
        }
    }

    // Draw hovering floor grid centered on hovered tile when in floor placement mode
    if let Some((snapped_x, snapped_z, _, _)) = preview_sector {
        if state.tool == EditorTool::DrawFloor {
            let inner_color = RasterColor::new(80, 180, 160); // Teal (bright)
            let outer_color = RasterColor::new(40, 90, 80);   // Teal (dim)

            let grid_y = 0.0;

            // Center of the hovered sector (snap to grid)
            let center_x = (snapped_x / SECTOR_SIZE).floor() * SECTOR_SIZE + SECTOR_SIZE * 0.5;
            let center_z = (snapped_z / SECTOR_SIZE).floor() * SECTOR_SIZE + SECTOR_SIZE * 0.5;

            let inner_half = SECTOR_SIZE * 1.5; // Inner 3x3
            let outer_half = SECTOR_SIZE * 2.5; // Outer 5x5

            // Draw grid lines - 6 lines in each direction for 5x5 grid
            for i in 0..=5 {
                let offset = -outer_half + (i as f32 * SECTOR_SIZE);
                let dist_from_center = offset.abs();

                let color = if dist_from_center <= inner_half {
                    inner_color
                } else {
                    outer_color
                };

                let z = center_z + offset;
                draw_3d_line(
                    fb,
                    Vec3::new(center_x - outer_half, grid_y, z),
                    Vec3::new(center_x + outer_half, grid_y, z),
                    &state.camera_3d,
                    color,
                );

                let x = center_x + offset;
                draw_3d_line(
                    fb,
                    Vec3::new(x, grid_y, center_z - outer_half),
                    Vec3::new(x, grid_y, center_z + outer_half),
                    &state.camera_3d,
                    color,
                );
            }
        }
    }

    // Draw hovering ceiling grid centered on hovered tile when in ceiling placement mode
    if let Some((snapped_x, snapped_z, _, _)) = preview_sector {
        if state.tool == EditorTool::DrawCeiling {
            use super::CEILING_HEIGHT;

            let inner_color = RasterColor::new(140, 100, 180); // Purple (bright)
            let outer_color = RasterColor::new(70, 50, 90);    // Purple (dim)

            let center_x = (snapped_x / SECTOR_SIZE).floor() * SECTOR_SIZE + SECTOR_SIZE * 0.5;
            let center_z = (snapped_z / SECTOR_SIZE).floor() * SECTOR_SIZE + SECTOR_SIZE * 0.5;

            let inner_half = SECTOR_SIZE * 1.5;
            let outer_half = SECTOR_SIZE * 2.5;

            for i in 0..=5 {
                let offset = -outer_half + (i as f32 * SECTOR_SIZE);
                let dist_from_center = offset.abs();

                let color = if dist_from_center <= inner_half {
                    inner_color
                } else {
                    outer_color
                };

                let z = center_z + offset;
                draw_3d_line(
                    fb,
                    Vec3::new(center_x - outer_half, CEILING_HEIGHT, z),
                    Vec3::new(center_x + outer_half, CEILING_HEIGHT, z),
                    &state.camera_3d,
                    color,
                );

                let x = center_x + offset;
                draw_3d_line(
                    fb,
                    Vec3::new(x, CEILING_HEIGHT, center_z - outer_half),
                    Vec3::new(x, CEILING_HEIGHT, center_z + outer_half),
                    &state.camera_3d,
                    color,
                );
            }
        }
    }

    // Build texture map from texture packs
    let mut texture_map: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
    let mut texture_idx = 0;
    for pack in &state.texture_packs {
        for tex in &pack.textures {
            texture_map.insert((pack.name.clone(), tex.name.clone()), texture_idx);
            texture_idx += 1;
        }
    }

    // Texture resolver closure
    let resolve_texture = |tex_ref: &crate::world::TextureRef| -> Option<usize> {
        if !tex_ref.is_valid() {
            return Some(0); // Fallback to first texture
        }
        texture_map.get(&(tex_ref.pack.clone(), tex_ref.name.clone())).copied()
    };

    // Render all rooms
    let settings = &state.raster_settings;
    for room in &state.level.rooms {
        let (vertices, faces) = room.to_render_data_with_textures(&resolve_texture);
        render_mesh(fb, &vertices, &faces, textures, &state.camera_3d, settings);
    }

    // Draw vertex overlays directly into framebuffer (only in Select mode)
    if state.tool == EditorTool::Select {
        for (world_pos, room_idx, gx, gz, corner_idx, _face) in &all_vertices {
            if let Some((fb_x, fb_y)) = world_to_screen(
                *world_pos,
                state.camera_3d.position,
                state.camera_3d.basis_x,
                state.camera_3d.basis_y,
                state.camera_3d.basis_z,
                fb.width,
                fb.height,
            ) {
                // Check if this specific vertex is hovered (match room, sector coords, and corner index)
                let is_hovered = hovered_vertex.map_or(false, |(hr, hgx, hgz, hci, _)|
                    hr == *room_idx && hgx == *gx && hgz == *gz && hci == *corner_idx);

                // Choose color based on state
                let color = if is_hovered {
                    RasterColor::new(255, 200, 150) // Orange when hovered
                } else {
                    RasterColor::with_alpha(200, 200, 220, 200) // Default (slightly transparent grey)
                };

                // Choose radius (larger when hovered)
                let radius = if is_hovered { 4 } else { 2 };

                // Draw circle directly into framebuffer
                fb.draw_circle(fb_x as i32, fb_y as i32, radius, color);
            }
        }
    }

    // Draw hovered edge highlight directly into framebuffer
    if let Some((room_idx, gx, gz, face_idx, edge_idx, _)) = hovered_edge {
        if let Some(room) = state.level.rooms.get(room_idx) {
            if let Some(sector) = room.get_sector(gx, gz) {
                let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
                let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;

                let edge_color = RasterColor::new(255, 200, 100); // Orange for edge hover

                // Get edge vertices based on face_idx
                let corners: Option<[Vec3; 4]> = match face_idx {
                    0 => sector.floor.as_ref().map(|f| [
                        Vec3::new(base_x, f.heights[0], base_z),
                        Vec3::new(base_x + SECTOR_SIZE, f.heights[1], base_z),
                        Vec3::new(base_x + SECTOR_SIZE, f.heights[2], base_z + SECTOR_SIZE),
                        Vec3::new(base_x, f.heights[3], base_z + SECTOR_SIZE),
                    ]),
                    1 => sector.ceiling.as_ref().map(|c| [
                        Vec3::new(base_x, c.heights[0], base_z),
                        Vec3::new(base_x + SECTOR_SIZE, c.heights[1], base_z),
                        Vec3::new(base_x + SECTOR_SIZE, c.heights[2], base_z + SECTOR_SIZE),
                        Vec3::new(base_x, c.heights[3], base_z + SECTOR_SIZE),
                    ]),
                    _ => None, // TODO: wall edges
                };

                if let Some(corners) = corners {
                    let v0 = corners[edge_idx];
                    let v1 = corners[(edge_idx + 1) % 4];

                    if let (Some((sx0, sy0)), Some((sx1, sy1))) = (
                        world_to_screen(v0, state.camera_3d.position, state.camera_3d.basis_x,
                            state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height),
                        world_to_screen(v1, state.camera_3d.position, state.camera_3d.basis_x,
                            state.camera_3d.basis_y, state.camera_3d.basis_z, fb.width, fb.height)
                    ) {
                        fb.draw_thick_line(sx0 as i32, sy0 as i32, sx1 as i32, sy1 as i32, 3, edge_color);
                    }
                }
            }
        }
    }

    // Draw hover highlight for hovered face (in Select mode)
    if let Some((room_idx, gx, gz, face)) = hovered_face {
        // Don't draw hover if this face is already selected
        let is_selected = state.selection.includes_face(room_idx, gx, gz, face);
        if !is_selected {
            if let Some(room) = state.level.rooms.get(room_idx) {
                if let Some(sector) = room.get_sector(gx, gz) {
                    let base_x = room.position.x + (gx as f32) * SECTOR_SIZE;
                    let base_z = room.position.z + (gz as f32) * SECTOR_SIZE;

                    let hover_color = RasterColor::new(150, 200, 255); // Light blue for hover

                    match face {
                        SectorFace::Floor => {
                            if let Some(floor) = &sector.floor {
                                let corners = [
                                    Vec3::new(base_x, floor.heights[0], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE),
                                    Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE),
                                ];
                                for i in 0..4 {
                                    draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, hover_color);
                                }
                                // Draw diagonal to show it's a face
                                draw_3d_line(fb, corners[0], corners[2], &state.camera_3d, hover_color);
                            }
                        }
                        SectorFace::Ceiling => {
                            if let Some(ceiling) = &sector.ceiling {
                                let corners = [
                                    Vec3::new(base_x, ceiling.heights[0], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE),
                                    Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE),
                                ];
                                for i in 0..4 {
                                    draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, hover_color);
                                }
                                draw_3d_line(fb, corners[0], corners[2], &state.camera_3d, hover_color);
                            }
                        }
                        SectorFace::WallNorth(i) => {
                            if let Some(wall) = sector.walls_north.get(i) {
                                let p0 = Vec3::new(base_x, wall.y_bottom, base_z);
                                let p1 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z);
                                let p2 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z);
                                let p3 = Vec3::new(base_x, wall.y_top, base_z);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, hover_color);
                            }
                        }
                        SectorFace::WallEast(i) => {
                            if let Some(wall) = sector.walls_east.get(i) {
                                let p0 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z);
                                let p1 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p2 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z + SECTOR_SIZE);
                                let p3 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, hover_color);
                            }
                        }
                        SectorFace::WallSouth(i) => {
                            if let Some(wall) = sector.walls_south.get(i) {
                                let p0 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p1 = Vec3::new(base_x, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p2 = Vec3::new(base_x, wall.y_top, base_z + SECTOR_SIZE);
                                let p3 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z + SECTOR_SIZE);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, hover_color);
                            }
                        }
                        SectorFace::WallWest(i) => {
                            if let Some(wall) = sector.walls_west.get(i) {
                                let p0 = Vec3::new(base_x, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p1 = Vec3::new(base_x, wall.y_bottom, base_z);
                                let p2 = Vec3::new(base_x, wall.y_top, base_z);
                                let p3 = Vec3::new(base_x, wall.y_top, base_z + SECTOR_SIZE);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, hover_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, hover_color);
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw selection highlight for selected face or sector
    match &state.selection {
        Selection::SectorFace { room, x, z, face } => {
            if let Some(room_data) = state.level.rooms.get(*room) {
                if let Some(sector) = room_data.get_sector(*x, *z) {
                    let base_x = room_data.position.x + (*x as f32) * SECTOR_SIZE;
                    let base_z = room_data.position.z + (*z as f32) * SECTOR_SIZE;

                    let select_color = RasterColor::new(255, 200, 80); // Yellow/orange

                    match face {
                        SectorFace::Floor => {
                            if let Some(floor) = &sector.floor {
                                let corners = [
                                    Vec3::new(base_x, floor.heights[0], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE),
                                    Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE),
                                ];
                                for i in 0..4 {
                                    draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, select_color);
                                }
                                draw_3d_line(fb, corners[0], corners[2], &state.camera_3d, select_color);
                            }
                        }
                        SectorFace::Ceiling => {
                            if let Some(ceiling) = &sector.ceiling {
                                let corners = [
                                    Vec3::new(base_x, ceiling.heights[0], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z),
                                    Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE),
                                    Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE),
                                ];
                                for i in 0..4 {
                                    draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, select_color);
                                }
                                draw_3d_line(fb, corners[0], corners[2], &state.camera_3d, select_color);
                            }
                        }
                        SectorFace::WallNorth(i) => {
                            if let Some(wall) = sector.walls_north.get(*i) {
                                let p0 = Vec3::new(base_x, wall.y_bottom, base_z);
                                let p1 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z);
                                let p2 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z);
                                let p3 = Vec3::new(base_x, wall.y_top, base_z);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, select_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, select_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, select_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, select_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, select_color);
                            }
                        }
                        SectorFace::WallEast(i) => {
                            if let Some(wall) = sector.walls_east.get(*i) {
                                let p0 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z);
                                let p1 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p2 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z + SECTOR_SIZE);
                                let p3 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, select_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, select_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, select_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, select_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, select_color);
                            }
                        }
                        SectorFace::WallSouth(i) => {
                            if let Some(wall) = sector.walls_south.get(*i) {
                                let p0 = Vec3::new(base_x + SECTOR_SIZE, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p1 = Vec3::new(base_x, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p2 = Vec3::new(base_x, wall.y_top, base_z + SECTOR_SIZE);
                                let p3 = Vec3::new(base_x + SECTOR_SIZE, wall.y_top, base_z + SECTOR_SIZE);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, select_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, select_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, select_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, select_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, select_color);
                            }
                        }
                        SectorFace::WallWest(i) => {
                            if let Some(wall) = sector.walls_west.get(*i) {
                                let p0 = Vec3::new(base_x, wall.y_bottom, base_z + SECTOR_SIZE);
                                let p1 = Vec3::new(base_x, wall.y_bottom, base_z);
                                let p2 = Vec3::new(base_x, wall.y_top, base_z);
                                let p3 = Vec3::new(base_x, wall.y_top, base_z + SECTOR_SIZE);
                                draw_3d_line(fb, p0, p1, &state.camera_3d, select_color);
                                draw_3d_line(fb, p1, p2, &state.camera_3d, select_color);
                                draw_3d_line(fb, p2, p3, &state.camera_3d, select_color);
                                draw_3d_line(fb, p3, p0, &state.camera_3d, select_color);
                                draw_3d_line(fb, p0, p2, &state.camera_3d, select_color);
                            }
                        }
                    }
                }
            }
        }
        Selection::Sector { room, x, z } => {
            // Sector-level selection (from 2D grid view) - highlight all faces
            if let Some(room_data) = state.level.rooms.get(*room) {
                if let Some(sector) = room_data.get_sector(*x, *z) {
                    let base_x = room_data.position.x + (*x as f32) * SECTOR_SIZE;
                    let base_z = room_data.position.z + (*z as f32) * SECTOR_SIZE;

                    let select_color = RasterColor::new(255, 200, 80); // Yellow/orange

                    // Draw floor outline if floor exists
                    if let Some(floor) = &sector.floor {
                        let corners = [
                            Vec3::new(base_x, floor.heights[0], base_z),
                            Vec3::new(base_x + SECTOR_SIZE, floor.heights[1], base_z),
                            Vec3::new(base_x + SECTOR_SIZE, floor.heights[2], base_z + SECTOR_SIZE),
                            Vec3::new(base_x, floor.heights[3], base_z + SECTOR_SIZE),
                        ];
                        for i in 0..4 {
                            draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, select_color);
                        }
                    }

                    // Draw ceiling outline if ceiling exists
                    if let Some(ceiling) = &sector.ceiling {
                        let corners = [
                            Vec3::new(base_x, ceiling.heights[0], base_z),
                            Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[1], base_z),
                            Vec3::new(base_x + SECTOR_SIZE, ceiling.heights[2], base_z + SECTOR_SIZE),
                            Vec3::new(base_x, ceiling.heights[3], base_z + SECTOR_SIZE),
                        ];
                        for i in 0..4 {
                            draw_3d_line(fb, corners[i], corners[(i + 1) % 4], &state.camera_3d, select_color);
                        }
                    }

                    // Draw vertical edges at corners
                    if sector.floor.is_some() || sector.ceiling.is_some() {
                        let floor_y = sector.floor.as_ref().map(|f| f.heights[0]).unwrap_or(0.0);
                        let ceiling_y = sector.ceiling.as_ref().map(|c| c.heights[0]).unwrap_or(1024.0);

                        let corner_positions = [
                            (base_x, base_z),
                            (base_x + SECTOR_SIZE, base_z),
                            (base_x + SECTOR_SIZE, base_z + SECTOR_SIZE),
                            (base_x, base_z + SECTOR_SIZE),
                        ];

                        for (i, &(cx, cz)) in corner_positions.iter().enumerate() {
                            let fy = sector.floor.as_ref().map(|f| f.heights[i]).unwrap_or(floor_y);
                            let cy = sector.ceiling.as_ref().map(|c| c.heights[i]).unwrap_or(ceiling_y);
                            draw_3d_line(
                                fb,
                                Vec3::new(cx, fy, cz),
                                Vec3::new(cx, cy, cz),
                                &state.camera_3d,
                                select_color,
                            );
                        }
                    }

                    // Draw wall outlines
                    let wall_sets = [
                        (&sector.walls_north, base_x, base_z, base_x + SECTOR_SIZE, base_z),
                        (&sector.walls_east, base_x + SECTOR_SIZE, base_z, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE),
                        (&sector.walls_south, base_x + SECTOR_SIZE, base_z + SECTOR_SIZE, base_x, base_z + SECTOR_SIZE),
                        (&sector.walls_west, base_x, base_z + SECTOR_SIZE, base_x, base_z),
                    ];

                    for (walls, x0, z0, x1, z1) in wall_sets {
                        for wall in walls {
                            let p0 = Vec3::new(x0, wall.y_bottom, z0);
                            let p1 = Vec3::new(x1, wall.y_bottom, z1);
                            let p2 = Vec3::new(x1, wall.y_top, z1);
                            let p3 = Vec3::new(x0, wall.y_top, z0);
                            draw_3d_line(fb, p0, p1, &state.camera_3d, select_color);
                            draw_3d_line(fb, p1, p2, &state.camera_3d, select_color);
                            draw_3d_line(fb, p2, p3, &state.camera_3d, select_color);
                            draw_3d_line(fb, p3, p0, &state.camera_3d, select_color);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Draw floor/ceiling placement preview wireframe with vertical sector boundaries
    if let Some((snapped_x, snapped_z, target_y, occupied)) = preview_sector {
        use super::CEILING_HEIGHT;

        let floor_y = 0.0;
        let ceiling_y = CEILING_HEIGHT;

        let corners = [
            Vec3::new(snapped_x, target_y, snapped_z),
            Vec3::new(snapped_x, target_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, target_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, target_y, snapped_z),
        ];

        let floor_corners = [
            Vec3::new(snapped_x, floor_y, snapped_z),
            Vec3::new(snapped_x, floor_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, floor_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, floor_y, snapped_z),
        ];

        let ceiling_corners = [
            Vec3::new(snapped_x, ceiling_y, snapped_z),
            Vec3::new(snapped_x, ceiling_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, ceiling_y, snapped_z + SECTOR_SIZE),
            Vec3::new(snapped_x + SECTOR_SIZE, ceiling_y, snapped_z),
        ];

        let mut screen_corners = Vec::new();
        let mut screen_floor = Vec::new();
        let mut screen_ceiling = Vec::new();

        for corner in &corners {
            if let Some((sx, sy)) = world_to_screen(*corner, state.camera_3d.position,
                state.camera_3d.basis_x, state.camera_3d.basis_y, state.camera_3d.basis_z,
                fb.width, fb.height)
            {
                screen_corners.push((sx as i32, sy as i32));
            }
        }

        for corner in &floor_corners {
            if let Some((sx, sy)) = world_to_screen(*corner, state.camera_3d.position,
                state.camera_3d.basis_x, state.camera_3d.basis_y, state.camera_3d.basis_z,
                fb.width, fb.height)
            {
                screen_floor.push((sx as i32, sy as i32));
            }
        }

        for corner in &ceiling_corners {
            if let Some((sx, sy)) = world_to_screen(*corner, state.camera_3d.position,
                state.camera_3d.basis_x, state.camera_3d.basis_y, state.camera_3d.basis_z,
                fb.width, fb.height)
            {
                screen_ceiling.push((sx as i32, sy as i32));
            }
        }

        // Green for valid placement, red for occupied
        let color = if occupied {
            RasterColor::new(255, 80, 80)
        } else {
            RasterColor::new(80, 255, 80)
        };
        let dim_color = if occupied {
            RasterColor::new(180, 60, 60)
        } else {
            RasterColor::new(60, 180, 60)
        };

        // Draw vertical boundary lines (floor to ceiling at each corner)
        if screen_floor.len() == 4 && screen_ceiling.len() == 4 {
            for i in 0..4 {
                let (fx, fy) = screen_floor[i];
                let (cx, cy) = screen_ceiling[i];
                fb.draw_line(fx, fy, cx, cy, dim_color);
            }

            for i in 0..4 {
                let (x0, y0) = screen_floor[i];
                let (x1, y1) = screen_floor[(i + 1) % 4];
                fb.draw_line(x0, y0, x1, y1, dim_color);
            }

            for i in 0..4 {
                let (x0, y0) = screen_ceiling[i];
                let (x1, y1) = screen_ceiling[(i + 1) % 4];
                fb.draw_line(x0, y0, x1, y1, dim_color);
            }
        }

        // Draw placement preview (the actual tile being placed - brighter)
        if screen_corners.len() == 4 {
            for i in 0..4 {
                let (x0, y0) = screen_corners[i];
                let (x1, y1) = screen_corners[(i + 1) % 4];
                fb.draw_thick_line(x0, y0, x1, y1, 2, color);
            }

            for (x, y) in &screen_corners {
                fb.draw_circle(*x, *y, 3, color);
            }
        }
    }

    // Convert framebuffer to texture and draw to viewport
    let texture = Texture2D::from_rgba8(fb.width as u16, fb.height as u16, &fb.pixels);
    texture.set_filter(FilterMode::Nearest);

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

    // Draw camera info (position and rotation)
    draw_text(
        &format!(
            "Cam: ({:.0}, {:.0}, {:.0}) | Rot: ({:.2}, {:.2})",
            state.camera_3d.position.x,
            state.camera_3d.position.y,
            state.camera_3d.position.z,
            state.camera_3d.rotation_x,
            state.camera_3d.rotation_y
        ),
        rect.x + 5.0,
        rect.bottom() - 5.0,
        14.0,
        Color::from_rgba(200, 200, 200, 255),
    );
}

/// Draw a 3D line into the framebuffer using Bresenham's algorithm
fn draw_3d_line(
    fb: &mut Framebuffer,
    p0: Vec3,
    p1: Vec3,
    camera: &crate::rasterizer::Camera,
    color: RasterColor,
) {
    const NEAR_PLANE: f32 = 0.1;

    // Transform to camera space
    let rel0 = p0 - camera.position;
    let rel1 = p1 - camera.position;

    let z0 = rel0.dot(camera.basis_z);
    let z1 = rel1.dot(camera.basis_z);

    // Both behind camera - skip entirely
    if z0 <= NEAR_PLANE && z1 <= NEAR_PLANE {
        return;
    }

    // Clip line to near plane if needed
    let (clipped_p0, clipped_p1) = if z0 <= NEAR_PLANE {
        let t = (NEAR_PLANE - z0) / (z1 - z0);
        let new_p0 = p0 + (p1 - p0) * t;
        (new_p0, p1)
    } else if z1 <= NEAR_PLANE {
        let t = (NEAR_PLANE - z0) / (z1 - z0);
        let new_p1 = p0 + (p1 - p0) * t;
        (p0, new_p1)
    } else {
        (p0, p1)
    };

    // Project clipped endpoints
    let s0 = world_to_screen(clipped_p0, camera.position, camera.basis_x, camera.basis_y, camera.basis_z, fb.width, fb.height);
    let s1 = world_to_screen(clipped_p1, camera.position, camera.basis_x, camera.basis_y, camera.basis_z, fb.width, fb.height);

    let (Some((x0f, y0f)), Some((x1f, y1f))) = (s0, s1) else {
        return;
    };

    // Convert to integers for Bresenham
    let mut x0 = x0f as i32;
    let mut y0 = y0f as i32;
    let x1 = x1f as i32;
    let y1 = y1f as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let w = fb.width as i32;
    let h = fb.height as i32;

    loop {
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            fb.set_pixel(x0 as usize, y0 as usize, color);
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
