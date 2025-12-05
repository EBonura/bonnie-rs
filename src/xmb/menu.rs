//! XMB Menu Data Structures
//!
//! Defines the structure for PS1-style XMB (Cross Media Bar) menu system

use serde::{Deserialize, Serialize};

/// Action to perform when an XMB item is selected
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum XMBAction {
    /// Do nothing
    None,
    /// Launch the level editor
    LaunchEditor,
    /// Launch the audio tracker
    LaunchTracker,
    /// Launch the game
    LaunchGame,
    /// Open settings menu
    OpenSettings,
    /// Load a recent level
    LoadRecentLevel(String),
    /// Exit the application
    Exit,
}

/// Icon types for XMB items (future: render as PS1-style sprites)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IconType {
    Editor,
    Tracker,
    Game,
    Settings,
    File,
    Audio,
}

/// A single item in the XMB menu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XMBItem {
    /// Display label
    pub label: String,
    /// Optional description shown when selected
    pub description: Option<String>,
    /// Action to perform when activated
    pub action: XMBAction,
    /// Optional icon type
    pub icon: Option<IconType>,
}

impl XMBItem {
    pub fn new(label: impl Into<String>, action: XMBAction) -> Self {
        Self {
            label: label.into(),
            description: None,
            action,
            icon: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_icon(mut self, icon: IconType) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// A category in the XMB menu (vertical column)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XMBCategory {
    /// Category label
    pub label: String,
    /// Items in this category
    pub items: Vec<XMBItem>,
    /// Optional icon type
    pub icon: Option<IconType>,
}

impl XMBCategory {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
            icon: None,
        }
    }

    pub fn with_icon(mut self, icon: IconType) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn add_item(mut self, item: XMBItem) -> Self {
        self.items.push(item);
        self
    }
}

/// Creates the default XMB menu structure
pub fn create_default_menu() -> Vec<XMBCategory> {
    vec![
        // Tools Category - All creation tools
        XMBCategory::new("Tools")
            .with_icon(IconType::Editor)
            .add_item(
                XMBItem::new("World Editor", XMBAction::LaunchEditor)
                    .with_description("Create and edit worlds with the integrated TRLE-style editor")
                    .with_icon(IconType::Editor)
            )
            .add_item(
                XMBItem::new("Sound Designer", XMBAction::LaunchTracker)
                    .with_description("Design instruments and sound effects")
                    .with_icon(IconType::Audio)
            )
            .add_item(
                XMBItem::new("Tracker", XMBAction::LaunchTracker)
                    .with_description("Picotron-style audio tracker and synthesizer")
                    .with_icon(IconType::Tracker)
            ),

        // Game Category
        XMBCategory::new("Game")
            .with_icon(IconType::Game)
            .add_item(
                XMBItem::new("Play", XMBAction::LaunchGame)
                    .with_description("Start playing the game")
                    .with_icon(IconType::Game)
            ),

        // Settings Category
        XMBCategory::new("Settings")
            .with_icon(IconType::Settings)
            .add_item(
                XMBItem::new("Options", XMBAction::OpenSettings)
                    .with_description("Graphics, controls, and engine settings")
                    .with_icon(IconType::Settings)
            ),
    ]
}
