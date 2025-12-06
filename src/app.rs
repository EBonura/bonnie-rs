//! Application state and tool management
//!
//! Fixed set of tools, each with its own persistent state.
//! Switch between tools via the tab bar - all tools stay alive in background.

use crate::editor::{EditorState, EditorLayout};
use crate::landing::LandingState;
use crate::tracker::TrackerState;
use crate::world::Level;
use macroquad::prelude::Font;
use std::path::PathBuf;

/// The available tools (fixed set, one tab each)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Home = 0,
    WorldEditor = 1,
    Tracker = 2,
}

impl Tool {
    pub const ALL: [Tool; 3] = [
        Tool::Home,
        Tool::WorldEditor,
        Tool::Tracker,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Tool::Home => "Home",
            Tool::WorldEditor => "World Editor",
            Tool::Tracker => "Music Editor",
        }
    }

    pub fn labels() -> [&'static str; 3] {
        [
            Tool::Home.label(),
            Tool::WorldEditor.label(),
            Tool::Tracker.label(),
        ]
    }

    pub fn from_index(i: usize) -> Option<Tool> {
        Tool::ALL.get(i).copied()
    }
}

/// State for the World Editor tool
pub struct WorldEditorState {
    pub editor_state: EditorState,
    pub editor_layout: EditorLayout,
}

/// Main application state containing all tool states
pub struct AppState {
    /// Currently active tool
    pub active_tool: Tool,

    /// Landing page state
    pub landing: LandingState,

    /// World Editor state
    pub world_editor: WorldEditorState,

    /// Music Editor state
    pub tracker: TrackerState,

    /// Icon font (Lucide)
    pub icon_font: Option<Font>,
}

impl AppState {
    /// Create new app state with the given initial level for the world editor
    pub fn new(level: Level, file_path: Option<PathBuf>, icon_font: Option<Font>) -> Self {
        let editor_state = if let Some(path) = file_path {
            EditorState::with_file(level, path)
        } else {
            EditorState::new(level)
        };

        Self {
            active_tool: Tool::Home,
            landing: LandingState::new(),
            world_editor: WorldEditorState {
                editor_state,
                editor_layout: EditorLayout::new(),
            },
            tracker: TrackerState::new(),
            icon_font,
        }
    }

    /// Switch to a different tool
    pub fn set_active_tool(&mut self, tool: Tool) {
        self.active_tool = tool;
    }

    /// Get the active tool index (for tab bar)
    pub fn active_tool_index(&self) -> usize {
        self.active_tool as usize
    }
}
