//! Level loading and saving
//!
//! Uses RON (Rusty Object Notation) for human-readable level files.

use std::fs;
use std::path::Path;
use super::Level;

/// Error type for level loading
#[derive(Debug)]
pub enum LevelError {
    IoError(std::io::Error),
    ParseError(ron::error::SpannedError),
    SerializeError(ron::Error),
}

impl From<std::io::Error> for LevelError {
    fn from(e: std::io::Error) -> Self {
        LevelError::IoError(e)
    }
}

impl From<ron::error::SpannedError> for LevelError {
    fn from(e: ron::error::SpannedError) -> Self {
        LevelError::ParseError(e)
    }
}

impl From<ron::Error> for LevelError {
    fn from(e: ron::Error) -> Self {
        LevelError::SerializeError(e)
    }
}

impl std::fmt::Display for LevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LevelError::IoError(e) => write!(f, "IO error: {}", e),
            LevelError::ParseError(e) => write!(f, "Parse error: {}", e),
            LevelError::SerializeError(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

/// Load a level from a RON file
pub fn load_level<P: AsRef<Path>>(path: P) -> Result<Level, LevelError> {
    let contents = fs::read_to_string(path)?;
    let mut level: Level = ron::from_str(&contents)?;

    // Recalculate bounds for all rooms (not serialized)
    for room in &mut level.rooms {
        room.recalculate_bounds();
    }

    Ok(level)
}

/// Save a level to a RON file
pub fn save_level<P: AsRef<Path>>(level: &Level, path: P) -> Result<(), LevelError> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .indentor("  ".to_string());

    let contents = ron::ser::to_string_pretty(level, config)?;
    fs::write(path, contents)?;
    Ok(())
}

/// Load a level from a RON string (for embedded levels or testing)
pub fn load_level_from_str(s: &str) -> Result<Level, LevelError> {
    let mut level: Level = ron::from_str(s)?;

    for room in &mut level.rooms {
        room.recalculate_bounds();
    }

    Ok(level)
}
