//! bonnie-rs: PS1-style software rasterizer engine
//!
//! A souls-like game engine with authentic PlayStation 1 rendering:
//! - Affine texture mapping (warpy textures)
//! - Vertex snapping (jittery vertices)
//! - Gouraud shading
//! - Low resolution (320x240)

mod rasterizer;

use macroquad::prelude::*;
use rasterizer::{
    Camera, Color, Framebuffer, RasterSettings, ShadingMode, Texture,
    create_test_cube, render_mesh, HEIGHT, WIDTH,
};

/// Convert our framebuffer to a macroquad texture
fn framebuffer_to_texture(fb: &Framebuffer) -> Texture2D {
    let texture = Texture2D::from_rgba8(fb.width as u16, fb.height as u16, &fb.pixels);
    texture.set_filter(FilterMode::Nearest); // No filtering - crispy pixels!
    texture
}

fn window_conf() -> Conf {
    Conf {
        window_title: "bonnie-rs :: PS1 Software Rasterizer".to_owned(),
        window_width: WIDTH as i32 * 3,  // 960x720 window
        window_height: HEIGHT as i32 * 3,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Initialize framebuffer
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);

    // Initialize camera
    let mut camera = Camera::new();
    camera.position = rasterizer::Vec3::new(0.0, 0.0, -5.0);

    // Create test cube
    let (vertices, faces) = create_test_cube();

    // Create test texture (checkerboard)
    let textures = vec![
        Texture::checkerboard(32, 32, Color::new(200, 100, 50), Color::new(50, 100, 200)),
    ];

    // Rasterizer settings
    let mut settings = RasterSettings::default();

    // Rotation for the cube
    let mut rotation = 0.0f32;

    // Mouse state for camera control
    let mut last_mouse_pos = mouse_position();
    let mut mouse_captured = false;

    println!("=== bonnie-rs ===");
    println!("Controls:");
    println!("  Right-click + drag: Look around");
    println!("  WASD: Move camera");
    println!("  1/2/3: Shading mode (None/Flat/Gouraud)");
    println!("  P: Toggle perspective correction");
    println!("  J: Toggle vertex jitter");
    println!("  Z: Toggle Z-buffer");
    println!("  Space: Pause rotation");
    println!("  ESC: Quit");

    let mut paused = false;

    loop {
        // Handle input
        // ESC only quits on native, not web (breaks WASM)
        #[cfg(not(target_arch = "wasm32"))]
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Toggle settings
        if is_key_pressed(KeyCode::Key1) {
            settings.shading = ShadingMode::None;
            println!("Shading: None");
        }
        if is_key_pressed(KeyCode::Key2) {
            settings.shading = ShadingMode::Flat;
            println!("Shading: Flat");
        }
        if is_key_pressed(KeyCode::Key3) {
            settings.shading = ShadingMode::Gouraud;
            println!("Shading: Gouraud");
        }
        if is_key_pressed(KeyCode::P) {
            settings.affine_textures = !settings.affine_textures;
            println!(
                "Textures: {}",
                if settings.affine_textures { "Affine (PS1)" } else { "Perspective-correct" }
            );
        }
        if is_key_pressed(KeyCode::J) {
            settings.vertex_snap = !settings.vertex_snap;
            println!(
                "Vertex snap: {}",
                if settings.vertex_snap { "ON (PS1 jitter)" } else { "OFF (smooth)" }
            );
        }
        if is_key_pressed(KeyCode::Z) {
            settings.use_zbuffer = !settings.use_zbuffer;
            println!(
                "Z-buffer: {}",
                if settings.use_zbuffer { "ON" } else { "OFF (painter's)" }
            );
        }
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
            println!("Rotation: {}", if paused { "PAUSED" } else { "RUNNING" });
        }

        // Camera rotation with right mouse button
        let mouse_pos = mouse_position();
        if is_mouse_button_down(MouseButton::Right) {
            if mouse_captured {
                let dx = (mouse_pos.1 - last_mouse_pos.1) * 0.005;
                let dy = (mouse_pos.0 - last_mouse_pos.0) * 0.005;
                camera.rotate(dx, dy);
            }
            mouse_captured = true;
        } else {
            mouse_captured = false;
        }
        last_mouse_pos = mouse_pos;

        // Camera movement (WASD)
        let move_speed = 0.1;
        if is_key_down(KeyCode::W) {
            camera.position = camera.position + camera.basis_z * move_speed;
        }
        if is_key_down(KeyCode::S) {
            camera.position = camera.position - camera.basis_z * move_speed;
        }
        if is_key_down(KeyCode::A) {
            camera.position = camera.position - camera.basis_x * move_speed;
        }
        if is_key_down(KeyCode::D) {
            camera.position = camera.position + camera.basis_x * move_speed;
        }

        // Update rotation
        if !paused {
            rotation += 0.02;
        }

        // Transform vertices for rotation (rotate around Y axis)
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let rotated_vertices: Vec<_> = vertices
            .iter()
            .map(|v| {
                let mut rv = *v;
                rv.pos = rasterizer::Vec3::new(
                    v.pos.x * cos_r - v.pos.z * sin_r,
                    v.pos.y,
                    v.pos.x * sin_r + v.pos.z * cos_r,
                );
                // Also rotate normal
                rv.normal = rasterizer::Vec3::new(
                    v.normal.x * cos_r - v.normal.z * sin_r,
                    v.normal.y,
                    v.normal.x * sin_r + v.normal.z * cos_r,
                );
                rv
            })
            .collect();

        // Clear framebuffer
        fb.clear(Color::new(20, 20, 30));

        // Render the cube
        render_mesh(&mut fb, &rotated_vertices, &faces, &textures, &camera, &settings);

        // Convert framebuffer to macroquad texture
        let texture = framebuffer_to_texture(&fb);

        // Draw to screen (scaled up)
        clear_background(BLACK);

        // Calculate scaled size maintaining aspect ratio
        let screen_w = screen_width();
        let screen_h = screen_height();
        let scale = (screen_w / WIDTH as f32).min(screen_h / HEIGHT as f32);
        let draw_w = WIDTH as f32 * scale;
        let draw_h = HEIGHT as f32 * scale;
        let draw_x = (screen_w - draw_w) / 2.0;
        let draw_y = (screen_h - draw_h) / 2.0;

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

        // Draw FPS
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, WHITE);

        next_frame().await;
    }
}
