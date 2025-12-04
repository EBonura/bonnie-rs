//! Vector math for 3D rendering
//! Ported from tipsy's C implementation

use std::ops::{Add, Sub, Mul};
use serde::{Serialize, Deserialize};

/// 3D Vector
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const UP: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let l = self.len();
        if l == 0.0 {
            return Vec3::ZERO;
        }
        Vec3 {
            x: self.x / l,
            y: self.y / l,
            z: self.z / l,
        }
    }

    pub fn scale(self, s: f32) -> Vec3 {
        Vec3 {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        self.scale(s)
    }
}

/// 2D Vector (for texture coordinates)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Transform a vertex by camera basis vectors (rotation)
pub fn perspective_transform(v: Vec3, cam_x: Vec3, cam_y: Vec3, cam_z: Vec3) -> Vec3 {
    Vec3 {
        x: v.dot(cam_x),
        y: v.dot(cam_y),
        z: v.dot(cam_z),
    }
}

/// Project a 3D point to 2D screen coordinates
/// If `snap` is true, coordinates are floored to integers (PS1 jitter effect)
pub fn project(v: Vec3, snap: bool, width: usize, height: usize) -> Vec3 {
    const DISTANCE: f32 = 5.0;
    const SCALE: f32 = 0.75;

    let ud = DISTANCE;
    let us = ud - 1.0;
    let vs = (width.min(height) as f32 / 2.0) * SCALE;

    // Perspective divide
    let denom = v.z + ud;
    if denom.abs() < 0.001 {
        return Vec3::new(width as f32 / 2.0, height as f32 / 2.0, DISTANCE);
    }

    let mut result = Vec3 {
        x: (v.x * us) / denom,
        y: (v.y * us) / denom,
        z: (v.z * us) / denom,
    };

    // Scale to screen
    result.x = result.x * vs + (width as f32 / 2.0);
    result.y = result.y * vs + (height as f32 / 2.0);
    result.z = result.z + DISTANCE;

    // PS1 vertex snapping
    if snap {
        result.x = result.x.floor();
        result.y = result.y.floor();
    }

    result
}

/// Calculate barycentric coordinates for point p in triangle (v1, v2, v3)
/// Returns (u, v, w) where u + v + w = 1 if point is inside triangle
pub fn barycentric(p: Vec3, v1: Vec3, v2: Vec3, v3: Vec3) -> Vec3 {
    let d = (v2.y - v3.y) * (v1.x - v3.x) + (v3.x - v2.x) * (v1.y - v3.y);

    if d.abs() < 0.0001 {
        return Vec3::new(-1.0, -1.0, -1.0); // Degenerate triangle
    }

    let u = ((v2.y - v3.y) * (p.x - v3.x) + (v3.x - v2.x) * (p.y - v3.y)) / d;
    let v = ((v3.y - v1.y) * (p.x - v3.x) + (v1.x - v3.x) * (p.y - v3.y)) / d;
    let w = 1.0 - u - v;

    Vec3::new(u, v, w)
}

/// Ray-triangle intersection using Möller–Trumbore algorithm
/// Returns Some(t) if ray hits, where t is the distance along the ray
/// ray_origin: starting point of ray
/// ray_dir: normalized direction of ray
/// v0, v1, v2: triangle vertices
pub fn ray_triangle_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    const EPSILON: f32 = 0.0000001;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_dir.cross(edge2);
    let a = edge1.dot(h);

    // Ray is parallel to triangle
    if a.abs() < EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_dir.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);

    if t > EPSILON {
        Some(t)
    } else {
        None
    }
}

/// Generate a ray from screen coordinates through the camera
/// Returns (ray_origin, ray_direction)
/// screen_x, screen_y: pixel coordinates
/// screen_width, screen_height: framebuffer dimensions
/// camera: the camera to cast from
pub fn screen_to_ray(
    screen_x: f32,
    screen_y: f32,
    screen_width: usize,
    screen_height: usize,
    cam_pos: Vec3,
    cam_x: Vec3,
    cam_y: Vec3,
    cam_z: Vec3,
) -> (Vec3, Vec3) {
    // Reverse the projection math from project()
    const DISTANCE: f32 = 5.0;
    const SCALE: f32 = 0.75;

    let vs = (screen_width.min(screen_height) as f32 / 2.0) * SCALE;

    // Convert screen coordinates to normalized device coordinates
    let ndc_x = (screen_x - screen_width as f32 / 2.0) / vs;
    let ndc_y = (screen_y - screen_height as f32 / 2.0) / vs;

    // The ray direction in camera space
    // At z=1 (unit distance in front of camera), the point would be at (ndc_x, ndc_y, 1)
    let cam_space_dir = Vec3::new(ndc_x, ndc_y, 1.0).normalize();

    // Transform ray direction from camera space to world space
    let world_dir = Vec3::new(
        cam_space_dir.x * cam_x.x + cam_space_dir.y * cam_y.x + cam_space_dir.z * cam_z.x,
        cam_space_dir.x * cam_x.y + cam_space_dir.y * cam_y.y + cam_space_dir.z * cam_z.y,
        cam_space_dir.x * cam_x.z + cam_space_dir.y * cam_y.z + cam_space_dir.z * cam_z.z,
    ).normalize();

    (cam_pos, world_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert!((a.dot(b) - 32.0).abs() < 0.001);
    }

    #[test]
    fn test_vec3_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(b);
        assert!((c.z - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_barycentric_inside() {
        let v1 = Vec3::new(0.0, 0.0, 0.0);
        let v2 = Vec3::new(10.0, 0.0, 0.0);
        let v3 = Vec3::new(5.0, 10.0, 0.0);
        let p = Vec3::new(5.0, 3.0, 0.0);
        let bc = barycentric(p, v1, v2, v3);
        assert!(bc.x >= 0.0 && bc.y >= 0.0 && bc.z >= 0.0);
    }
}
