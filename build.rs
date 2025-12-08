//! Build script to generate manifests for WASM builds
//!
//! Scans assets/textures/ and assets/levels/ and creates manifests
//! listing all files, since WASM can't enumerate directories at runtime.

use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/textures");
    println!("cargo:rerun-if-changed=assets/levels");

    generate_texture_manifest();
    generate_levels_manifest();
}

/// Generate manifest for texture packs
fn generate_texture_manifest() {
    let textures_dir = Path::new("assets/textures");
    let manifest_path = Path::new("assets/textures/manifest.txt");

    let mut manifest = String::new();

    if textures_dir.exists() {
        let mut packs: Vec<_> = fs::read_dir(textures_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        packs.sort_by_key(|e| e.file_name());

        for pack_entry in packs {
            let pack_path = pack_entry.path();
            let pack_name = pack_entry.file_name().to_string_lossy().to_string();

            // Get all PNG files in the pack
            let mut textures: Vec<_> = fs::read_dir(&pack_path)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext.to_ascii_lowercase() == "png")
                        .unwrap_or(false)
                })
                .collect();

            textures.sort_by_key(|e| e.file_name());

            if !textures.is_empty() {
                // Pack header: [pack_name]
                manifest.push_str(&format!("[{}]\n", pack_name));

                for tex_entry in textures {
                    let tex_name = tex_entry.file_name().to_string_lossy().to_string();
                    manifest.push_str(&format!("{}\n", tex_name));
                }

                manifest.push('\n');
            }
        }
    }

    // Write manifest file
    let mut file = fs::File::create(manifest_path).unwrap();
    file.write_all(manifest.as_bytes()).unwrap();
}

/// Generate manifest for levels (for WASM builds)
fn generate_levels_manifest() {
    let levels_dir = Path::new("assets/levels");
    let manifest_path = Path::new("assets/levels/manifest.txt");

    let mut manifest = String::new();

    if levels_dir.exists() {
        let mut levels: Vec<_> = fs::read_dir(levels_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                // Only include .ron files, skip directories
                path.is_file() && path
                    .extension()
                    .map(|ext| ext.to_ascii_lowercase() == "ron")
                    .unwrap_or(false)
            })
            .collect();

        levels.sort_by_key(|e| e.file_name());

        for level_entry in levels {
            let level_name = level_entry.file_name().to_string_lossy().to_string();
            manifest.push_str(&format!("{}\n", level_name));
        }
    }

    // Write manifest file
    let mut file = fs::File::create(manifest_path).unwrap();
    file.write_all(manifest.as_bytes()).unwrap();
}
